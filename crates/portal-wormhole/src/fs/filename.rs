use std::ffi::{OsStr, OsString};
use std::path::Path;

pub fn sanitize_untrusted_filename<'a>(
    file_path: &'a Path,
    fallback_file_stem: &'a OsStr,
    fallback_extension: &'a OsStr,
) -> OsString {
    let stem = file_path
        .file_stem()
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_file_stem);
    let extension = file_path
        .extension()
        .filter(|e| !e.is_empty())
        .unwrap_or(fallback_extension);
    let mut filename = OsString::with_capacity(stem.len() + extension.len() + 1);
    filename.push(stem);
    filename.push(".");
    filename.push(extension);
    filename
}
