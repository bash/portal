use std::path::PathBuf;
use std::sync::Arc;

use super::sendable_file::SendableFile;

#[derive(Clone, Debug)]
pub enum SendRequest {
    File(PathBuf),
    Folder(PathBuf),
    Selection(Vec<PathBuf>),
    Cached(Box<SendRequest>, CachedSendRequest),
}

impl SendRequest {
    pub(crate) fn new_cached(
        sendable_file: Arc<SendableFile>,
        original_request: SendRequest,
    ) -> SendRequest {
        SendRequest::Cached(original_request.flatten(), CachedSendRequest(sendable_file))
    }

    fn flatten(self) -> Box<SendRequest> {
        match self {
            SendRequest::Cached(inner_request, _) => inner_request,
            _ => Box::new(self),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CachedSendRequest(pub(crate) Arc<crate::send::SendableFile>);

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
