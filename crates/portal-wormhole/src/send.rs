use crate::error::PortalError;
use crate::temp_zip::{pack_folder_as_zip, pack_selection_as_zip};
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
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::NamedTempFile;

#[derive(Clone, Debug)]
pub enum SendRequest {
    File(PathBuf),
    Folder(PathBuf),
    Selection(Vec<PathBuf>),
}

impl SendRequest {
    pub fn from_paths(paths: Vec<PathBuf>) -> Option<Self> {
        match paths.len() {
            0 => None,
            1 if paths[0].is_dir() => Some(SendRequest::Folder(paths[0].clone())),
            1 => Some(SendRequest::File(paths[0].clone())),
            _ => Some(SendRequest::Selection(paths)),
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
    let (cancel_sender, cancel_receiver) = oneshot::channel();
    let (abort_handle, abort_registration) = AbortHandle::new_pair();

    let controller = SendingController {
        progress_receiver,
        cancel_sender: Some(cancel_sender),
        abort_handle,
    };

    let future = send_impl(
        send_request,
        report(progress_updater, request_repaint),
        async { _ = cancel_receiver.await },
        abort_registration,
    );

    (future, controller)
}

pub struct SendingController {
    progress_receiver: svc::Receiver<SendingProgress>,
    cancel_sender: Option<oneshot::Sender<()>>,
    abort_handle: AbortHandle,
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
    }
}

async fn send_impl(
    send_request: SendRequest,
    mut report: impl Reporter,
    cancel: impl Future<Output = ()>,
    abort_registration: AbortRegistration,
) -> Result<(), PortalError> {
    let (transit_info_receiver, transit_info_updater) = svc::channel();

    report(SendingProgress::Packing);
    let sendable_file = SendableFile::from_send_request(send_request)?;

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

enum SendableFile {
    Path(PathBuf),
    Temporary(NamedTempFile, OsString),
}

impl SendableFile {
    fn from_send_request(send_request: SendRequest) -> Result<SendableFile, PortalError> {
        match send_request {
            SendRequest::File(file_path) => Ok(SendableFile::Path(file_path)),
            SendRequest::Folder(folder_path) => Ok(SendableFile::Temporary(
                pack_folder_as_zip(&folder_path)?,
                folder_zip_file_name(&folder_path),
            )),
            SendRequest::Selection(paths) => Ok(SendableFile::Temporary(
                pack_selection_as_zip(&paths)?,
                selection_zip_file_name(&paths),
            )),
        }
    }

    fn path(&self) -> &Path {
        match self {
            SendableFile::Path(path) => path,
            SendableFile::Temporary(file, _) => file.path(),
        }
    }

    fn file_name(&self) -> &OsStr {
        match self {
            SendableFile::Path(path) => path.file_name().unwrap(),
            SendableFile::Temporary(_, file_name) => file_name,
        }
    }
}

fn folder_zip_file_name(folder_path: &Path) -> OsString {
    folder_path
        .file_name()
        .map(|p| concat_os_strs(p, ".zip"))
        .unwrap_or_else(|| OsString::from("Folder.zip"))
}

fn selection_zip_file_name(paths: &[PathBuf]) -> OsString {
    common_parent_directory(paths)
        .and_then(|p| p.file_name())
        .map(|p| concat_os_strs(p, ".zip"))
        .unwrap_or_else(|| OsString::from("Selection.zip"))
}

fn concat_os_strs(a: impl AsRef<OsStr>, b: impl AsRef<OsStr>) -> OsString {
    let a = a.as_ref();
    let b = b.as_ref();
    let mut result = OsString::with_capacity(a.len() + b.len());
    result.push(a);
    result.push(b);
    result
}

fn common_parent_directory(paths: &[PathBuf]) -> Option<&Path> {
    let parent = paths.first()?.parent()?;
    paths
        .iter()
        .skip(1)
        .all(|p| p.parent() == Some(parent))
        .then_some(parent)
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
