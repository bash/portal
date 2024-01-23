use super::path_parts::PathParts;
use std::io::{self, ErrorKind};
use std::path::Path;
use tailcall::tailcall;

pub fn open_with_conflict_resolution<T>(
    path: &Path,
    opener: impl FnMut(&Path) -> io::Result<T>,
) -> io::Result<T> {
    return open(PathParts::try_from(path).expect("Invalid path"), opener, 0);

    #[tailcall]
    fn open<T>(
        path_parts: PathParts,
        mut opener: impl FnMut(&Path) -> io::Result<T>,
        counter: u64,
    ) -> io::Result<T> {
        let path = path_parts.to_path_with_counter(counter);
        match opener(&path) {
            Err(error) if error.kind() == ErrorKind::AlreadyExists => open(
                path_parts,
                opener,
                counter.checked_add(1).expect("Counter overflow"),
            ),
            result => result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::path::PathBuf;
    use thiserror::Error;

    #[test]
    fn uses_original_file_name_when_possible() {
        let expected_path = PathBuf::from("bar/foo.txt");
        let path = open_with_conflict_resolution(&expected_path, open(VecDeque::default()))
            .expect("open to succeed");
        assert_eq!(path, expected_path);
    }

    #[test]
    fn retries_on_conflict() {
        let first_path = PathBuf::from("bar/foo.txt");
        let existing_paths = vec![
            first_path.clone(),
            PathBuf::from("bar/foo (1).txt"),
            PathBuf::from("bar/foo (2).txt"),
            PathBuf::from("bar/foo (3).txt"),
        ];
        let expected_path = PathBuf::from("bar/foo (4).txt");
        let path = open_with_conflict_resolution(&first_path, open(existing_paths.into()))
            .expect("open to succeed");
        assert_eq!(path, expected_path);
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
