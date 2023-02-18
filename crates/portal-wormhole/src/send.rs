use self::sendable_file::SendableFile;
use crate::error::PortalError;
use crate::sync::{cancellation_pair, CancellationReceiver, CancellationSender};
use crate::transit::{ProgressHandler, TransitHandler, RELAY_HINTS};
use crate::{Progress, RequestRepaint};
use async_std::fs::File;
use futures::channel::oneshot;
use futures::future::{AbortHandle, AbortRegistration, Abortable, BoxFuture};
use futures::Future;
use magic_wormhole::Code;
use magic_wormhole::{
    transfer,
    transit::{Abilities, TransitInfo},
    Wormhole, WormholeWelcome,
};
use single_value_channel as svc;
use std::sync::Arc;

mod request;
pub use self::request::SendRequest;
mod sendable_file;

pub fn send(
    send_request: SendRequest,
    request_repaint: impl RequestRepaint,
) -> (
    impl Future<Output = Result<(), PortalError>>,
    SendingController,
) {
    let (progress_receiver, progress_updater) =
        svc::channel_starting_with(SendingProgress::Connecting);
    let (cancel_sender, cancel_receiver) = oneshot::channel();
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let (pack_abort_handle, pack_abort_registration) = AbortHandle::new_pair();
    let (cancellation_sender, cancellation_receiver) = cancellation_pair();

    let controller = SendingController {
        progress_receiver,
        cancel_sender: Some(cancel_sender),
        abort_handle,
        pack_abort_handle,
        cancellation_sender,
    };

    let future = send_impl(
        send_request,
        report(progress_updater, request_repaint),
        async { _ = cancel_receiver.await },
        abort_registration,
        pack_abort_registration,
        cancellation_receiver,
    );

    (future, controller)
}

pub struct SendingController {
    progress_receiver: svc::Receiver<SendingProgress>,
    cancel_sender: Option<oneshot::Sender<()>>,
    abort_handle: AbortHandle,
    pack_abort_handle: AbortHandle,
    cancellation_sender: CancellationSender,
}

pub enum SendingProgress {
    Packing,
    Connecting,
    Connected(Code),
    PreparingToSend,
    Sending(Arc<TransitInfo>, Progress),
}

impl SendingController {
    pub fn progress(&mut self) -> &SendingProgress {
        self.progress_receiver.latest()
    }

    pub fn cancel(&mut self) {
        self.abort_handle.abort();
        self.cancel_sender.take().map(|c| c.send(()));
        self.pack_abort_handle.abort();
        self.cancellation_sender.cancel();
    }
}

async fn send_impl(
    send_request: SendRequest,
    mut report: impl Reporter,
    cancel: impl Future<Output = ()>,
    abort_registration: AbortRegistration,
    pack_abort_registration: AbortRegistration,
    cancellation_receiver: CancellationReceiver,
) -> Result<(), PortalError> {
    let (transit_info_receiver, transit_info_updater) = svc::channel();

    report(SendingProgress::Packing);
    let sendable_file = Abortable::new(
        SendableFile::from_send_request(send_request, cancellation_receiver),
        pack_abort_registration,
    )
    .await??;

    report(SendingProgress::Connecting);
    let wormhole = async {
        let (welcome, wormhole_future) = connect().await?;
        report(SendingProgress::Connected(welcome.code));

        let wormhole = wormhole_future.await?;
        report(SendingProgress::PreparingToSend);

        Result::<_, PortalError>::Ok(wormhole)
    };

    let wormhole = Abortable::new(wormhole, abort_registration).await??;

    send_file(
        wormhole,
        sendable_file,
        progress_handler(transit_info_receiver, report.clone()),
        transit_handler(transit_info_updater, report),
        cancel,
    )
    .await
}

trait Reporter = FnMut(SendingProgress) + Clone + 'static;

fn report(
    updater: svc::Updater<SendingProgress>,
    mut request_repaint: impl RequestRepaint,
) -> impl Reporter {
    move |progress| {
        _ = updater.update(progress);
        request_repaint();
    }
}

fn transit_handler(
    updater: svc::Updater<Option<Arc<TransitInfo>>>,
    mut report: impl Reporter,
) -> impl TransitHandler {
    move |transit_info, _| {
        let transit_info = Arc::new(transit_info);
        _ = updater.update(Some(Arc::clone(&transit_info)));
        report(SendingProgress::Sending(transit_info, Progress::default()));
    }
}

fn progress_handler(
    mut transit_info: svc::Receiver<Option<Arc<TransitInfo>>>,
    mut report: impl Reporter,
) -> impl ProgressHandler {
    move |value, total| {
        let transit_info = transit_info.latest().clone().unwrap();
        report(SendingProgress::Sending(
            transit_info,
            Progress { value, total },
        ))
    }
}

async fn send_file(
    wormhole: Wormhole,
    sendable_file: SendableFile,
    progress_handler: impl ProgressHandler,
    transit_handler: impl TransitHandler,
    cancel: impl Future<Output = ()>,
) -> Result<(), PortalError> {
    let mut file = File::open(sendable_file.path()).await?;
    let metadata = file.metadata().await?;
    let file_size = metadata.len();

    let mut canceled = false;
    transfer::send_file(
        wormhole,
        RELAY_HINTS.clone(),
        &mut file,
        sendable_file.file_name(),
        file_size,
        Abilities::ALL_ABILITIES,
        transit_handler,
        progress_handler,
        async {
            cancel.await;
            canceled = true;
        },
    )
    .await?;

    if canceled {
        Err(PortalError::Canceled)
    } else {
        Ok(())
    }
}

async fn connect() -> Result<
    (
        WormholeWelcome,
        BoxFuture<'static, Result<Wormhole, PortalError>>,
    ),
    PortalError,
> {
    let (welcome, future) = Wormhole::connect_without_code(transfer::APP_CONFIG, 4).await?;
    Ok((welcome, Box::pin(async { Ok(future.await?) })))
}
