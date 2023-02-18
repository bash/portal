use super::SendRequest;
use crate::temp_zip::{pack_folder_as_zip, pack_selection_as_zip};
use crate::PortalError;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub enum SendableFile {
    Path(PathBuf),
    Temporary(NamedTempFile, OsString),
}

impl SendableFile {
    pub fn from_send_request(send_request: SendRequest) -> Result<SendableFile, PortalError> {
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

    pub fn path(&self) -> &Path {
        match self {
            SendableFile::Path(path) => path,
            SendableFile::Temporary(file, _) => file.path(),
        }
    }

    pub fn file_name(&self) -> &OsStr {
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
