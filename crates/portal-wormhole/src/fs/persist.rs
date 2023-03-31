use super::Filename;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use tailcall::tailcall;
use tempfile::NamedTempFile;

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
    path: PathBuf,
    filename: Filename,
    save: impl FnMut(T, &Path) -> PersistResult<T>,
) -> io::Result<PathBuf> {
    return persist(state, path, filename, save, 0);

    #[tailcall]
    fn persist<T>(
        state: T,
        mut path: PathBuf,
        filename: Filename,
        mut save: impl FnMut(T, &Path) -> PersistResult<T>,
        counter: u64,
    ) -> io::Result<PathBuf> {
        path.set_file_name(filename.with_counter(counter).to_os_string());
        match save(state, &path) {
            PersistResult::Conflict(state) => persist(
                state,
                path,
                filename,
                save,
                counter.checked_add(1).expect("Counter overflow"),
            ),
            PersistResult::Ok => Ok(path),
            PersistResult::Err(error) => Err(error),
        }
    }
}
