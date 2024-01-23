use super::SendRequest;
use crate::cancellation::CancellationToken;
use crate::temp_zip::{pack_folder_as_zip, pack_selection_as_zip};
use crate::PortalError;
use async_std::task::spawn_blocking;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::NamedTempFile;

#[derive(Debug)]
pub(crate) enum SendableFile {
    Path(PathBuf),
    Temporary(OsString, NamedTempFile),
}

impl SendableFile {
    /// Note that cancelling this future may not cancel the background work
    /// immediately as the packing functions only accept cancellation in between files.
    pub(crate) async fn from_send_request(
        send_request: SendRequest,
        cancellation: CancellationToken,
    ) -> Result<Arc<SendableFile>, PortalError> {
        match send_request {
            SendRequest::Cached(_, cached) => Ok(cached.0),
            SendRequest::File(file_path) => Ok(Arc::new(SendableFile::Path(file_path))),
            SendRequest::Folder(folder_path) => Ok(Arc::new(SendableFile::Temporary(
                folder_zip_file_name(&folder_path),
                spawn_blocking(move || pack_folder_as_zip(&folder_path, cancellation)).await?,
            ))),
            SendRequest::Selection(paths) => Ok(Arc::new(SendableFile::Temporary(
                selection_zip_file_name(&paths),
                spawn_blocking(move || pack_selection_as_zip(&paths, cancellation)).await?,
            ))),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        match self {
            SendableFile::Path(path) => path,
            SendableFile::Temporary(_, file) => file.path(),
        }
    }

    pub(crate) fn file_name(&self) -> &OsStr {
        match self {
            SendableFile::Path(path) => path.file_name().expect("path should be absolute"),
            SendableFile::Temporary(file_name, _) => file_name,
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
