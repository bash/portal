use crate::cancellation::CancellationToken;
use crate::PortalError;
use std::borrow::Cow;
use std::fs::{self, File};
use std::io::{self, Seek, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

/// Packs a folder as a Zip file recursively.
pub(crate) fn pack_folder_as_zip(
    folder_path: &Path,
    cancellation: CancellationToken,
) -> Result<NamedTempFile, PortalError> {
    cancellation.error_if_canceled()?;

    let mut temp_file = NamedTempFile::new()?;
    {
        let mut writer = ZipWriter::new(&mut temp_file);
        add_folder_to_zip(folder_path, None, &mut writer, cancellation)?;
    }
    Ok(temp_file)
}

/// Packs a selection of paths (e.g. from drag and drop) as a Zip file.
///
/// Note that this function does not handle duplicate entries as this should be a rare case
/// (it requires selecting files across multiple directories).
pub(crate) fn pack_selection_as_zip(
    paths: &[PathBuf],
    cancellation: CancellationToken,
) -> Result<NamedTempFile, PortalError> {
    cancellation.error_if_canceled()?;

    let mut temp_file = NamedTempFile::new()?;
    {
        let mut writer = ZipWriter::new(&mut temp_file);
        for path in paths {
            add_path_to_zip(path, None, &mut writer, cancellation.clone())?;
        }
    }
    Ok(temp_file)
}

/// Adds a file or folder to the Zip file.
///
/// Symbolic links are materialized (i.e. resolved and the real files or folders are added to the Zip file).
fn add_path_to_zip<W>(
    path: &Path,
    relative_path: Option<&Path>,
    writer: &mut ZipWriter<W>,
    cancellation: CancellationToken,
) -> Result<(), PortalError>
where
    W: Write + Seek,
{
    cancellation.error_if_canceled()?;

    let relative_path = relative_path
        .unwrap_or_else(|| Path::new(path.file_name().expect("path should be absolute")));

    if path.is_dir() {
        add_folder_to_zip(path, Some(relative_path), writer, cancellation)?;
    } else if path.is_file() {
        add_file_to_zip(path, relative_path, writer)?;
    } else if path.is_symlink() {
        add_path_to_zip(
            &std::fs::read_link(path)?,
            Some(relative_path),
            writer,
            cancellation,
        )?;
    } else {
        unreachable!("Path is either a file, a directory or a symlink");
    }

    Ok(())
}

/// Adds a folder to the Zip file by recursively walking through the directory.
///
/// Symbolic links are materialized (i.e. resolved and the real files are added to the Zip file). \
/// This is the default behaviour of the `zip` tool and best for cross-platform compatibility.
fn add_folder_to_zip<W>(
    folder_path: &Path,
    folder_relative_path: Option<&Path>,
    writer: &mut ZipWriter<W>,
    cancellation: CancellationToken,
) -> Result<(), PortalError>
where
    W: Write + Seek,
{
    cancellation.error_if_canceled()?;

    for entry in WalkDir::new(folder_path).follow_links(true) {
        cancellation.error_if_canceled()?;

        let entry = entry?;
        let relative_path =
            relative_path_for_entry_in_folder(folder_path, folder_relative_path, entry.path());
        let relative_path_as_string = relative_path.to_string_lossy();

        if entry.file_type().is_dir() {
            writer.add_directory(relative_path_as_string, FileOptions::default())?;
        } else if entry.file_type().is_file() {
            add_file_to_zip(entry.path(), &relative_path, writer)?;
        } else {
            unreachable!("The file is either a file or directory. Symlinks have been be materialized by .follow_links(true)");
        }
    }

    Ok(())
}

fn relative_path_for_entry_in_folder<'a>(
    folder_path: &'a Path,
    folder_relative_path: Option<&'a Path>,
    entry_path: &'a Path,
) -> Cow<'a, Path> {
    let relative_path = entry_path
        .strip_prefix(folder_path)
        .expect("File in folder should start with folder path");
    match folder_relative_path {
        None => Cow::Borrowed(relative_path),
        Some(folder_relative_path) => {
            let mut combined_path = PathBuf::new();
            combined_path.push(folder_relative_path);
            combined_path.push(relative_path);
            Cow::Owned(combined_path)
        }
    }
}

fn add_file_to_zip<W>(
    source_path: &Path,
    relative_path: &Path,
    writer: &mut ZipWriter<W>,
) -> Result<(), PortalError>
where
    W: Write + Seek,
{
    writer.start_file(relative_path.to_string_lossy(), file_options(source_path)?)?;
    let mut reader = File::open(source_path)?;
    io::copy(&mut reader, writer)?;
    Ok(())
}

fn file_options(path: &Path) -> Result<FileOptions, PortalError> {
    // Files >= 4 GiB require large_file
    let file_size = fs::metadata(path)?.len();
    let large_file = file_size > u32::MAX as u64;

    Ok(FileOptions::default().large_file(large_file))
}
