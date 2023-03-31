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
    mut path: PathBuf,
    filename: Filename,
    save: impl FnMut(T, &Path) -> PersistResult<T>,
) -> io::Result<PathBuf> {
    path.push(filename.to_os_string());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::sanitize_untrusted_filename;
    use std::collections::VecDeque;

    #[test]
    fn uses_original_file_name_when_possible() {
        let expected_path = PathBuf::from("bar/foo.txt");
        let filename = filename_from_path("foo.txt".as_ref());
        let path = persist_with_conflict_resolution(
            VecDeque::default(),
            PathBuf::from("bar"),
            filename,
            save,
        )
        .unwrap();
        assert_eq!(path, expected_path);
    }

    #[test]
    fn retries_on_conflict() {
        let existing_paths = vec![
            PathBuf::from("bar/foo.txt"),
            PathBuf::from("bar/foo (1).txt"),
            PathBuf::from("bar/foo (2).txt"),
            PathBuf::from("bar/foo (3).txt"),
        ];
        let expected_path = PathBuf::from("bar/foo (4).txt");
        let filename = filename_from_path("foo.txt".as_ref());
        let path = persist_with_conflict_resolution(
            existing_paths.into(),
            PathBuf::from("bar"),
            filename,
            save,
        )
        .unwrap();
        assert_eq!(path, expected_path);
    }

    fn filename_from_path(path: &Path) -> Filename<'_> {
        sanitize_untrusted_filename(path, "fallback_stem".as_ref(), "fallback_ext".as_ref())
    }

    fn save(
        mut existing_paths: VecDeque<PathBuf>,
        path: &Path,
    ) -> PersistResult<VecDeque<PathBuf>> {
        match existing_paths.pop_front() {
            None => PersistResult::Ok,
            Some(expected_path) => {
                assert_eq!(path, expected_path);
                PersistResult::Conflict(existing_paths)
            }
        }
    }
}
