use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

pub struct Filename<'a> {
    stem: Cow<'a, OsStr>,
    extension: &'a OsStr,
}

impl<'a> Filename<'a> {
    fn with_counter(&self, counter: u64) -> Filename<'a> {
        let mut stem = self.stem.to_owned().into_owned();
        stem.push(&format!(" ({counter})"));
        Filename {
            stem: Cow::Owned(stem),
            extension: &self.extension,
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

pub fn sanitize_untrusted_file_path<'a>(
    file_path: &'a Path,
    fallback_file_stem: &'a OsStr,
    fallback_extension: &'a OsStr,
) -> Filename<'a> {
    let stem = file_path
        .file_stem()
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_file_stem.as_ref());
    let extension = file_path
        .extension()
        .filter(|e| !e.is_empty())
        .unwrap_or(fallback_extension.as_ref());
    Filename {
        stem: Cow::Borrowed(stem),
        extension,
    }
}

pub fn save_with_conflict_resolution(
    mut path: PathBuf,
    filename: Filename,
    mut save: impl FnMut(&Path) -> std::io::Result<()>,
) -> std::io::Result<PathBuf> {
    path.push(filename.to_os_string());

    let mut result = save(&path);

    let mut counter = 1;
    while let Err(ref error) = result && error.kind() == std::io::ErrorKind::AlreadyExists {
        path.set_file_name(filename.with_counter(counter).to_os_string());
        result = save(&path);
        counter += 1;
    }

    Ok(path)
}
