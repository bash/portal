use std::path::PathBuf;

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
