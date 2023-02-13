use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub struct Filename<'a> {
    stem: Cow<'a, OsStr>,
    extension: &'a OsStr,
}

impl<'a> Filename<'a> {
    fn with_counter(&self, counter: u64) -> Filename<'a> {
        let mut stem = self.stem.clone().into_owned();
        stem.push(&format!(" ({counter})"));
        Filename {
            stem: Cow::Owned(stem),
            extension: self.extension,
        }
    }

    fn to_os_string(&self) -> OsString {
        let mut filename = OsString::with_capacity(self.stem.len() + self.extension.len() + 1);
        filename.push(&self.stem);
        filename.push(".");
        filename.push(self.extension);
        filename
    }
}

pub fn sanitize_untrusted_filename<'a>(
    file_path: &'a Path,
    fallback_file_stem: &'a OsStr,
    fallback_extension: &'a OsStr,
) -> Filename<'a> {
    let stem = file_path
        .file_stem()
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_file_stem);
    let extension = file_path
        .extension()
        .filter(|e| !e.is_empty())
        .unwrap_or(fallback_extension);
    Filename {
        stem: Cow::Borrowed(stem),
        extension,
    }
}

pub enum PersistResult<T> {
    Ok,
    Conflict(T),
    Err(io::Error),
}

pub fn persist_temp_file(file: NamedTempFile, path: &Path) -> PersistResult<NamedTempFile> {
    to_persist_result(file.persist_noclobber(path))
}

fn to_persist_result<T>(result: Result<T, tempfile::PersistError>) -> PersistResult<NamedTempFile> {
    match result {
        Ok(_) => PersistResult::Ok,
        Err(error) if error.error.kind() == ErrorKind::AlreadyExists => {
            PersistResult::Conflict(error.file)
        }
        Err(error) => PersistResult::Err(error.error),
    }
}

pub fn persist_with_conflict_resolution<T>(
    state: T,
    mut path: PathBuf,
    filename: Filename,
    mut save: impl FnMut(T, &Path) -> PersistResult<T>,
) -> io::Result<PathBuf> {
    path.push(filename.to_os_string());

    let mut result = save(state, &path);

    let mut counter = 1;
    while let PersistResult::Conflict(state) = result {
        path.set_file_name(filename.with_counter(counter).to_os_string());
        result = save(state, &path);
        counter += 1;
    }

    match result {
        PersistResult::Conflict(_) => unreachable!(),
        PersistResult::Ok => Ok(path),
        PersistResult::Err(error) => Err(error),
    }
}
