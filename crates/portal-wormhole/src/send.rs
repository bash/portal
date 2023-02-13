use crate::error::PortalError;
use crate::transit::{ProgressHandler, TransitHandler, RELAY_HINTS};
use crate::{Progress, RequestRepaint};
use async_std::fs::File;
use futures::future::BoxFuture;
use futures::Future;
use magic_wormhole::Code;
use magic_wormhole::{
    transfer,
    transit::{Abilities, TransitInfo},
    Wormhole, WormholeWelcome,
};
use single_value_channel as svc;
use std::sync::Arc;
use std::{
    future,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub enum SendRequest {
    File(PathBuf),
    Folder(PathBuf),
}

impl SendRequest {
    pub fn path(&self) -> &Path {
        match self {
            SendRequest::File(path) => path,
            SendRequest::Folder(path) => path,
        }
    }
}

pub fn send(
    send_request: SendRequest,
    request_repaint: impl RequestRepaint,
) -> (
    impl Future<Output = Result<(), PortalError>>,
    SendingController,
) {
    let (progress_receiver, progress_updater) =
        svc::channel_starting_with(SendingProgress::Connecting);

    let controller = SendingController { progress_receiver };
    let future = send_impl(send_request, report(progress_updater, request_repaint));

    (future, controller)
}

async fn send_impl(
    send_request: SendRequest,
    mut report: impl Reporter,
) -> Result<(), PortalError> {
    let (transit_info_receiver, transit_info_updater) = svc::channel();

    let (welcome, wormhole_future) = connect().await?;
    report(SendingProgress::Connected(welcome.code));

    let wormhole = wormhole_future.await?;
    report(SendingProgress::PreparingToSend);

    match send_request {
        SendRequest::File(file_path) => {
            send_file(
                wormhole,
                file_path.clone(),
                progress_handler(transit_info_receiver, report.clone()),
                transit_handler(transit_info_updater, report),
            )
            .await
        }
        SendRequest::Folder(folder_path) => {
            send_folder(
                wormhole,
                folder_path.clone(),
                progress_handler(transit_info_receiver, report.clone()),
                transit_handler(transit_info_updater, report),
            )
            .await
        }
    }
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

pub struct SendingController {
    progress_receiver: svc::Receiver<SendingProgress>,
}

pub enum SendingProgress {
    Connecting,
    Connected(Code),
    PreparingToSend,
    Sending(Arc<TransitInfo>, Progress),
}

impl SendingController {
    pub fn progress(&mut self) -> &SendingProgress {
        self.progress_receiver.latest()
    }
}

async fn send_file(
    wormhole: Wormhole,
    path: PathBuf,
    progress_handler: impl ProgressHandler,
    transit_handler: impl TransitHandler,
) -> Result<(), PortalError> {
    let mut file = File::open(&path).await?;
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    transfer::send_file(
        wormhole,
        RELAY_HINTS.clone(),
        &mut file,
        path.file_name().unwrap(),
        file_size,
        Abilities::ALL_ABILITIES,
        transit_handler,
        progress_handler,
        future::pending(),
    )
    .await?;
    Ok(())
}

async fn send_folder(
    wormhole: Wormhole,
    path: PathBuf,
    progress_handler: impl ProgressHandler,
    transit_handler: impl TransitHandler,
) -> Result<(), PortalError> {
    transfer::send_folder(
        wormhole,
        RELAY_HINTS.clone(),
        &path,
        path.file_name().unwrap(),
        Abilities::ALL_ABILITIES,
        transit_handler,
        progress_handler,
        future::pending(),
    )
    .await?;
    Ok(())
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
