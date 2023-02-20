use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use super::Filename;

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
