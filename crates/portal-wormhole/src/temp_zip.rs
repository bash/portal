use crate::PortalError;
use std::fs::File;
use std::io::{self, Seek, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

pub fn pack_folder_as_zip(folder_path: &Path) -> Result<NamedTempFile, PortalError> {
    let mut temp_file = NamedTempFile::new()?;
    {
        let mut writer = ZipWriter::new(&mut temp_file);
        add_folder_to_zip(folder_path, &mut writer)?;
    }
    Ok(temp_file)
}

pub fn pack_selection_as_zip(paths: &[PathBuf]) -> Result<NamedTempFile, PortalError> {
    let mut temp_file = NamedTempFile::new()?;
    {
        let mut writer = ZipWriter::new(&mut temp_file);
        for path in paths {
            if path.is_dir() {
                add_folder_to_zip(path, &mut writer)?;
            } else if path.is_file() {
                add_file_to_zip(path, Path::new(path.file_name().unwrap()), &mut writer)?;
            } else if path.is_symlink() {
                todo!()
            }
        }
    }
    Ok(temp_file)
}

fn add_folder_to_zip<W>(folder_path: &Path, writer: &mut ZipWriter<W>) -> Result<(), PortalError>
where
    W: Write + Seek,
{
    for entry in WalkDir::new(folder_path) {
        let entry = entry?;
        let relative_path = entry
            .path()
            .strip_prefix(folder_path)
            .expect("File in folder should start with folder path");
        let relative_path_as_string = relative_path.to_string_lossy();

        if entry.file_type().is_dir() {
            writer.add_directory(relative_path_as_string, FileOptions::default())?;
        } else if entry.file_type().is_file() {
            add_file_to_zip(entry.path(), relative_path, writer)?;
        } else if entry.path_is_symlink() {
            todo!()
        }
    }

    Ok(())
}

fn add_file_to_zip<W>(
    source_path: &Path,
    relative_path: &Path,
    writer: &mut ZipWriter<W>,
) -> Result<(), PortalError>
where
    W: Write + Seek,
{
    writer.start_file(relative_path.to_string_lossy(), FileOptions::default())?;
    let mut reader = File::open(source_path)?;
    io::copy(&mut reader, writer)?;
    Ok(())
}
