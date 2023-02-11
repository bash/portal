use std::process::Command;
use std::{io, path::Path};

#[cfg(target_os = "windows")]
pub fn open_file_in_folder(path: impl AsRef<Path>) -> Result<(), io::Error> {
    Command::new("explorer.exe")
        .arg("/select,")
        .arg(path.as_ref())
        .output()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn open_file_in_folder(path: impl AsRef<Path>) -> Result<(), io::Error> {
    Command::new("open")
        .arg("-R")
        .arg("--")
        .arg(path.as_ref())
        .output()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn open_file_in_folder(_path: impl AsRef<Path>) -> Result<(), io::Error> {
    // TODO: Support linux: https://gitlab.gnome.org/World/pika-backup/-/blob/main/src/ui/page_archives/display.rs#L63
    unimplemented!()
}