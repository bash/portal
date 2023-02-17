use crate::PortalError;
use std::fs::File;
use std::io::{self, Seek, Write};
use std::path::Path;
use tempfile::NamedTempFile;
use walkdir::WalkDir;
use zip::write::FileOptions;
use zip::ZipWriter;

// TODO: Progress report
pub fn pack_folder_as_zip(folder_path: &Path) -> Result<NamedTempFile, PortalError> {
    let mut temp_file = NamedTempFile::new()?;
    add_folder_to_zip(folder_path, &mut temp_file)?;
    Ok(temp_file)
}

fn add_folder_to_zip<W>(folder_path: &Path, write: &mut W) -> Result<(), PortalError>
where
    W: Write + Seek,
{
    let mut writer = ZipWriter::new(write);

    for entry in WalkDir::new(folder_path) {
        let entry = entry?;
        let relative_path = entry
            .path()
            .strip_prefix(folder_path)
            .expect("File in folder should start with folder path");
        let relative_path_as_string = relative_path.to_string_lossy();

        // TODO: decide when to enable large_file(true)

        if entry.file_type().is_dir() {
            writer.add_directory(relative_path_as_string, FileOptions::default())?;
        } else if entry.file_type().is_file() {
            writer.start_file(relative_path_as_string, FileOptions::default())?;
            let mut reader = File::open(entry.path())?;
            io::copy(&mut reader, &mut writer)?;
        } else if entry.path_is_symlink() {
            todo!()
        }
    }

    Ok(())
}
