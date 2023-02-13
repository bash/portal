use crate::defaults::RELAY_HINTS;
use crate::error::PortalError;
use crate::sync::BorrowingOneshotReceiver;
use crate::RequestRepaint;
use async_std::fs::File;
use futures::future::BoxFuture;
use magic_wormhole::{
    transfer,
    transit::{Abilities, TransitInfo},
    Wormhole, WormholeWelcome,
};
use single_value_channel as svc;
use std::{
    future,
    path::{Path, PathBuf},
};

pub type ConnectResult = Result<
    (
        WormholeWelcome,
        BoxFuture<'static, Result<Wormhole, PortalError>>,
    ),
    PortalError,
>;

#[derive(Default, Copy, Clone)]
pub struct Progress {
    pub sent: u64,
    pub total: u64,
}

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

pub struct SendingController {
    transit_info_receiver: BorrowingOneshotReceiver<TransitInfo>,
    progress_receiver: svc::Receiver<Progress>,
}

impl SendingController {
    pub fn transit_info(&mut self) -> Option<&TransitInfo> {
        self.transit_info_receiver.value()
    }

    pub fn progress(&mut self) -> Progress {
        *self.progress_receiver.latest()
    }
}

impl SendingController {
    // TODO: this function needs refactoring
    pub fn new(
        send_request: &SendRequest,
        wormhole: Wormhole,
        request_repaint: impl RequestRepaint,
    ) -> (BoxFuture<'static, Result<(), PortalError>>, Self) {
        let (progress_receiver, progress_updater) = svc::channel_starting_with(Progress::default());
        let (transit_sender, transit_info_receiver) = oneshot::channel();
        let future = {
            let send_request = send_request.clone();
            async {
                match send_request {
                    SendRequest::File(file_path) => {
                        send_file(
                            wormhole,
                            file_path.clone(),
                            progress_updater,
                            transit_sender,
                            request_repaint,
                        )
                        .await
                    }
                    SendRequest::Folder(folder_path) => {
                        send_folder(
                            wormhole,
                            folder_path.clone(),
                            progress_updater,
                            transit_sender,
                            request_repaint,
                        )
                        .await
                    }
                }
            }
        };
        let controller = SendingController {
            transit_info_receiver: transit_info_receiver.into(),
            progress_receiver,
        };
        (Box::pin(future), controller)
    }
}

async fn send_file(
    wormhole: Wormhole,
    path: PathBuf,
    progress: svc::Updater<Progress>,
    transit_info_sender: oneshot::Sender<TransitInfo>,
    mut request_repaint: impl RequestRepaint,
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
        {
            let mut request_repaint = request_repaint.clone();
            move |transit_info, _| {
                _ = transit_info_sender.send(transit_info);
                request_repaint();
            }
        },
        move |sent, total| {
            _ = progress.update(Progress { sent, total });
            request_repaint();
        },
        future::pending(),
    )
    .await?;
    Ok(())
}

async fn send_folder(
    wormhole: Wormhole,
    path: PathBuf,
    progress: svc::Updater<Progress>,
    transit_info_sender: oneshot::Sender<TransitInfo>,
    mut request_repaint: impl RequestRepaint,
) -> Result<(), PortalError> {
    transfer::send_folder(
        wormhole,
        RELAY_HINTS.clone(),
        &path,
        path.file_name().unwrap(),
        Abilities::ALL_ABILITIES,
        {
            let mut request_repaint = request_repaint.clone();
            move |transit_info, _| {
                _ = transit_info_sender.send(transit_info);
                request_repaint();
            }
        },
        move |sent, total| {
            _ = progress.update(Progress { sent, total });
            request_repaint();
        },
        future::pending(),
    )
    .await?;
    Ok(())
}
