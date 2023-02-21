#[cfg(target_os = "windows")]
pub use self::windows::*;

#[cfg(target_os = "macos")]
pub use self::macos::*;

#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "windows")]
mod windows {
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub fn reveal(path: impl AsRef<Path>) -> Result<(), io::Error> {
        Command::new("explorer.exe")
            .arg("/select,")
            .arg(path.as_ref())
            .spawn()?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::io;
    use std::path::Path;
    use std::process::Command;

    pub fn reveal(path: impl AsRef<Path>) -> Result<(), io::Error> {
        Command::new("open")
            .arg("-R")
            .arg("--")
            .arg(path.as_ref())
            .spawn()?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::io;
    use std::path::Path;

    pub fn reveal(_path: impl AsRef<Path>) -> Result<(), io::Error> {
        // TODO: Support linux: https://gitlab.gnome.org/World/pika-backup/-/blob/main/src/ui/page_archives/display.rs#L63
        unimplemented!()
    }
}