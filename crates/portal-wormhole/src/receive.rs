use crate::transit::{transit_handler, TransitHandler, RELAY_HINTS};
use crate::{
    error::PortalError,
    fs::{persist_temp_file, persist_with_conflict_resolution, sanitize_untrusted_filename},
    sync::BorrowingOneshotReceiver,
};
use crate::{Progress, RequestRepaint};
use async_std::fs::File;
use futures::future::{AbortHandle, AbortRegistration, Abortable};
use futures::{channel::oneshot, Future};
use magic_wormhole::{
    transfer::{self, ReceiveRequest},
    transit::{Abilities, TransitInfo},
    Code, Wormhole,
};
use single_value_channel as svc;
use std::path::{Path, PathBuf};

pub type ConnectResult = Result<ReceiveRequestController, PortalError>;
pub type ReceiveResult = Result<PathBuf, PortalError>;

pub fn connect(code: Code) -> (impl Future<Output = ConnectResult>, ConnectingController) {
    let (wormhole_abort_handle, abort_registration) = AbortHandle::new_pair();
    let (cancel_sender, cancel_receiver) = oneshot::channel();
    let cancel_future = async { _ = cancel_receiver.await };
    let controller = ConnectingController {
        cancel_sender: Some(cancel_sender),
        wormhole_abort_handle,
    };
    (
        connect_impl(code, abort_registration, cancel_future),
        controller,
    )
}

pub struct ConnectingController {
    wormhole_abort_handle: AbortHandle,
    cancel_sender: Option<oneshot::Sender<()>>,
}

impl ConnectingController {
    pub fn cancel(&mut self) {
        self.cancel_sender.take().map(|c| c.send(()));
        self.wormhole_abort_handle.abort();
    }
}

async fn connect_impl(
    code: Code,
    abort_registration: AbortRegistration,
    cancel: impl Future<Output = ()>,
) -> ConnectResult {
    let (_, wormhole) = Abortable::new(
        Wormhole::connect_with_code(transfer::APP_CONFIG, code),
        abort_registration,
    )
    .await??;

    transfer::request_file(
        wormhole,
        RELAY_HINTS.clone(),
        Abilities::ALL_ABILITIES,
        cancel,
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
            transit_handler(transit_info_sender, request_repaint),
            progress_updater,
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
    progress_updater: svc::Updater<Progress>,
    cancel: oneshot::Receiver<()>,
) -> ReceiveResult {
    let temp_file = tempfile::NamedTempFile::new()?;
    let mut temp_file_async = File::from(temp_file.reopen()?);

    let untrusted_filename = receive_request.filename.clone();

    let mut canceled = false;
    receive_request
        .accept(
            transit_handler,
            move |value, total| {
                _ = progress_updater.update(Progress { value, total });
            },
            &mut temp_file_async,
            async {
                _ = cancel.await;
                canceled = true;
            },
        )
        .await?;
    if canceled {
        return Err(PortalError::Canceled);
    }

    let file_name = sanitize_untrusted_filename(
        &untrusted_filename,
        "Downloaded File".as_ref(),
        "bin".as_ref(),
    );
    let persisted_path = persist_with_conflict_resolution(
        temp_file,
        dirs::download_dir().expect("Unable to detect downloads directory"),
        file_name,
        persist_temp_file,
    )?;

    Ok(persisted_path)
}
