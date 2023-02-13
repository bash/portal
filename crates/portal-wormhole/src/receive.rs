use crate::{
    error::PortalError,
    fs::{persist_temp_file, persist_with_conflict_resolution, sanitize_untrusted_filename},
    sync::BorrowingOneshotReceiver,
};
use async_std::fs::File;
use futures::future::{AbortHandle, AbortRegistration, Abortable};
use futures::{channel::oneshot, Future};
use magic_wormhole::{
    transfer::{self, ReceiveRequest},
    transit::{self, Abilities, TransitInfo},
    Code, Wormhole,
};
use single_value_channel as svc;
use std::path::PathBuf;

pub type ConnectResult = Result<ReceiveRequest, PortalError>;
pub type ReceiveResult = Result<PathBuf, PortalError>;

#[derive(Default, Copy, Clone)]
pub struct ReceiveProgress {
    pub received: u64,
    pub total: u64,
}

pub struct ReceivingController {
    transit_info_receiver: BorrowingOneshotReceiver<TransitInfo>,
    progress: svc::Receiver<ReceiveProgress>,
    cancel_sender: Option<oneshot::Sender<()>>,
}

impl ReceivingController {
    pub fn new(receive_request: ReceiveRequest) -> (impl Future<Output = ReceiveResult>, Self) {
        let (transit_info_sender, transit_info_receiver) = ::oneshot::channel();
        let (progress, progress_updater) = svc::channel_starting_with(ReceiveProgress::default());
        let (cancel_sender, cancel_receiver) = oneshot::channel();
        let controller = ReceivingController {
            transit_info_receiver: transit_info_receiver.into(),
            progress,
            cancel_sender: Some(cancel_sender),
        };
        let future = accept(
            receive_request,
            transit_info_sender,
            progress_updater,
            cancel_receiver,
        );
        (future, controller)
    }

    pub fn transit_info(&mut self) -> Option<&TransitInfo> {
        self.transit_info_receiver.value()
    }

    pub fn progress(&mut self) -> &ReceiveProgress {
        self.progress.latest()
    }

    pub fn cancel(&mut self) {
        self.cancel_sender.take().map(|c| c.send(()));
    }
}

async fn accept(
    receive_request: ReceiveRequest,
    transit_info_sender: ::oneshot::Sender<TransitInfo>,
    progress_updater: svc::Updater<ReceiveProgress>,
    cancel: oneshot::Receiver<()>,
) -> ReceiveResult {
    let temp_file = tempfile::NamedTempFile::new()?;
    let mut temp_file_async = File::from(temp_file.reopen()?);

    let untrusted_filename = receive_request.filename.clone();

    let mut canceled = false;
    receive_request
        .accept(
            |transit_info, _| {
                _ = transit_info_sender.send(transit_info);
            },
            move |received, total| {
                _ = progress_updater.update(ReceiveProgress { received, total });
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

pub struct ConnectingController {
    wormhole_abort_handle: AbortHandle,
    cancel_sender: Option<oneshot::Sender<()>>,
}

impl ConnectingController {
    pub fn new(code: Code) -> (impl Future<Output = ConnectResult>, Self) {
        let (wormhole_abort_handle, abort_registration) = AbortHandle::new_pair();
        let (cancel_sender, cancel_receiver) = oneshot::channel();
        let cancel_future = async { _ = cancel_receiver.await };
        let controller = ConnectingController {
            cancel_sender: Some(cancel_sender),
            wormhole_abort_handle,
        };
        (connect(code, abort_registration, cancel_future), controller)
    }

    pub fn cancel(&mut self) {
        self.cancel_sender.take().map(|c| c.send(()));
        self.wormhole_abort_handle.abort();
    }
}

async fn connect(
    code: Code,
    abort_registration: AbortRegistration,
    cancel: impl Future<Output = ()>,
) -> ConnectResult {
    let (_, wormhole) = Abortable::new(
        Wormhole::connect_with_code(transfer::APP_CONFIG, code),
        abort_registration,
    )
    .await??;

    let relay_hint =
        transit::RelayHint::from_urls(None, [transit::DEFAULT_RELAY_SERVER.parse().unwrap()])
            .unwrap();
    transfer::request_file(wormhole, vec![relay_hint], Abilities::ALL_ABILITIES, cancel)
        .await?
        .ok_or(PortalError::Canceled)
}
