use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::path::Path;

#[derive(Clone)]
pub struct Filename<'a> {
    stem: Cow<'a, OsStr>,
    extension: &'a OsStr,
}

impl<'a> Filename<'a> {
    pub fn with_counter(&self, counter: u64) -> Filename<'a> {
        if counter == 0 {
            self.clone()
        } else {
            let mut stem = self.stem.clone().into_owned();
            stem.push(format!(" ({counter})"));
            Filename {
                stem: Cow::Owned(stem),
                extension: self.extension,
            }
        }
    }

    pub fn to_os_string(&self) -> OsString {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn counter_is_added_to_file_stem() {
        let path = PathBuf::from("foo.txt");
        let filename = filename_from_path(&path);
        let filename_with_counter = filename.with_counter(13);
        assert_eq!(filename_with_counter.to_os_string(), "foo (13).txt");
    }

    #[test]
    fn filename_is_left_untouched_when_counter_is_zero() {
        let path = PathBuf::from("foo.txt");
        let filename = filename_from_path(&path);
        let filename_with_counter = filename.with_counter(0);
        assert_eq!(path, filename_with_counter.to_os_string());
    }

    fn filename_from_path(path: &Path) -> Filename<'_> {
        Filename {
            stem: Cow::Borrowed(path.file_stem().unwrap()),
            extension: path.extension().unwrap(),
        }
    }
}
