use super::Filename;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use tailcall::tailcall;

pub fn open_with_conflict_resolution<T>(
    mut path: PathBuf,
    filename: Filename,
    opener: impl FnMut(&Path) -> io::Result<T>,
) -> io::Result<T> {
    path.push(filename.to_os_string());
    return open(path, filename, opener, 0);

    #[tailcall]
    fn open<T>(
        mut path: PathBuf,
        filename: Filename,
        mut opener: impl FnMut(&Path) -> io::Result<T>,
        counter: u64,
    ) -> io::Result<T> {
        path.set_file_name(filename.with_counter(counter).to_os_string());
        match opener(&path) {
            Err(error) if error.kind() == ErrorKind::AlreadyExists => open(
                path,
                filename,
                opener,
                counter.checked_add(1).expect("Counter overflow"),
            ),
            result => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use thiserror::Error;

    use super::*;
    use crate::fs::sanitize_untrusted_filename;
    use std::collections::VecDeque;

    #[test]
    fn uses_original_file_name_when_possible() {
        let expected_path = PathBuf::from("bar/foo.txt");
        let filename = filename_from_path("foo.txt".as_ref());
        let path = open_with_conflict_resolution(
            PathBuf::from("bar"),
            filename,
            open(VecDeque::default()),
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
        let path = open_with_conflict_resolution(
            PathBuf::from("bar"),
            filename,
            open(existing_paths.into()),
        )
        .unwrap();
        assert_eq!(path, expected_path);
    }

    fn filename_from_path(path: &Path) -> Filename<'_> {
        sanitize_untrusted_filename(path, "fallback_stem".as_ref(), "fallback_ext".as_ref())
    }

    fn open(mut existing_paths: VecDeque<PathBuf>) -> impl FnMut(&Path) -> io::Result<PathBuf> {
        move |path| match existing_paths.pop_front() {
            None => Ok(path.to_owned()),
            Some(expected_path) => {
                assert_eq!(path, expected_path);
                Err(io::Error::new(ErrorKind::AlreadyExists, UnitError))
            }
        }
    }

    #[derive(Error, Debug)]
    #[error("")]
    struct UnitError;
}
