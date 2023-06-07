use crate::cancellation::{CancellationSource, CancellationToken};
use crate::error::PortalError;
use crate::fs::{mark_as_downloaded, open_with_conflict_resolution, sanitize_untrusted_filename};
use crate::sync::BorrowingOneshotReceiver;
use crate::transit::{
    progress_handler, transit_handler, ProgressHandler, TransitHandler, RELAY_HINTS,
};
use crate::{Progress, RequestRepaint};
use async_std::fs::File;
use futures::channel::oneshot;
use futures::future::Abortable;
use futures::Future;
use magic_wormhole::transfer::{self, ReceiveRequest};
use magic_wormhole::transit::{Abilities, TransitInfo};
use magic_wormhole::{Code, Wormhole};
use single_value_channel as svc;
use std::fs::{self, OpenOptions};
use std::mem;
use std::path::{Path, PathBuf};

pub type ConnectResult = Result<ReceiveRequestController, PortalError>;
pub type ReceiveResult = Result<PathBuf, PortalError>;

pub fn connect(code: Code) -> (impl Future<Output = ConnectResult>, ConnectingController) {
    let cancellation_source = CancellationSource::default();
    let cancellation_token = cancellation_source.token();
    let controller = ConnectingController {
        cancellation_source,
    };
    (connect_impl(code, cancellation_token), controller)
}

pub struct ConnectingController {
    cancellation_source: CancellationSource,
}

impl ConnectingController {
    pub fn cancel(&mut self) {
        self.cancellation_source.cancel()
    }
}

async fn connect_impl(code: Code, cancellation: CancellationToken) -> ConnectResult {
    let (_, wormhole) = Abortable::new(
        Wormhole::connect_with_code(transfer::APP_CONFIG, code),
        cancellation.as_abort_registration(),
    )
    .await??;

    transfer::request_file(
        wormhole,
        RELAY_HINTS.clone(),
        Abilities::ALL_ABILITIES,
        cancellation.as_future(),
    )
    .await?
    .ok_or(PortalError::Canceled)
    .map(|receive_request| ReceiveRequestController { receive_request })
}

pub struct ReceiveRequestController {
    receive_request: ReceiveRequest,
}

impl ReceiveRequestController {
    pub fn filename(&self) -> &Path {
        &self.receive_request.filename
    }

    pub fn filesize(&self) -> u64 {
        self.receive_request.filesize
    }

    pub fn accept(
        self,
        request_repaint: impl RequestRepaint,
    ) -> (impl Future<Output = ReceiveResult>, ReceivingController) {
        ReceivingController::new(self.receive_request, request_repaint)
    }

    pub async fn reject(self) -> Result<(), PortalError> {
        Ok(self.receive_request.reject().await?)
    }
}

pub struct ReceivingController {
    transit_info_receiver: BorrowingOneshotReceiver<TransitInfo>,
    progress: svc::Receiver<Progress>,
    cancel_sender: Option<oneshot::Sender<()>>,
}

impl ReceivingController {
    fn new(
        receive_request: ReceiveRequest,
        request_repaint: impl RequestRepaint,
    ) -> (impl Future<Output = ReceiveResult>, Self) {
        let (transit_info_sender, transit_info_receiver) = ::oneshot::channel();
        let (progress, progress_updater) = svc::channel_starting_with(Progress::default());
        let (cancel_sender, cancel_receiver) = oneshot::channel();
        let controller = ReceivingController {
            transit_info_receiver: transit_info_receiver.into(),
            progress,
            cancel_sender: Some(cancel_sender),
        };
        let future = accept(
            receive_request,
            transit_handler(transit_info_sender, request_repaint.clone()),
            progress_handler(progress_updater, request_repaint),
            cancel_receiver,
        );
        (future, controller)
    }

    pub fn transit_info(&mut self) -> Option<&TransitInfo> {
        self.transit_info_receiver.value()
    }

    pub fn progress(&mut self) -> &Progress {
        self.progress.latest()
    }

    pub fn cancel(&mut self) {
        self.cancel_sender.take().map(|c| c.send(()));
    }
}

async fn accept(
    receive_request: ReceiveRequest,
    transit_handler: impl TransitHandler,
    progress_handler: impl ProgressHandler,
    cancel: oneshot::Receiver<()>,
) -> ReceiveResult {
    let untrusted_filename = receive_request.filename.clone();
    let file_name = sanitize_untrusted_filename(
        &untrusted_filename,
        "Downloaded File".as_ref(),
        "bin".as_ref(),
    );

    let base_path = {
        let mut path = dirs::download_dir().expect("Unable to detect downloads directory");
        path.push(file_name);
        path
    };
    let (file, file_path) = open_with_conflict_resolution(&base_path, |path| {
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map(|f| (f, path.to_owned()))
    })?;
    let mut async_file = File::from(file);

    let mut canceled = false;
    receive_request
        .accept(transit_handler, progress_handler, &mut async_file, async {
            _ = cancel.await;
            canceled = true;
        })
        .await?;

    if canceled {
        mem::drop(async_file);
        fs::remove_file(file_path)?;
        return Err(PortalError::Canceled);
    }

    mark_as_downloaded(&file_path);

    Ok(file_path)
}
