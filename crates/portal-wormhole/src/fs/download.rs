#[cfg(not(target_os = "windows"))]
pub use self::generic::*;

#[cfg(target_os = "windows")]
pub use self::windows::*;

#[cfg(not(target_os = "windows"))]
mod generic {
    use std::path::Path;

    pub fn mark_as_downloaded(_path: &Path) {}
}

mod macos {
    //! On macOS the file is marked as quarantined, because we have
    //! `LSFileQuarantineEnabled` set to `true` in our app's `Info.plist`.
    //!
    //! See: <https://ilostmynotes.blogspot.com/2012/06/gatekeeper-xprotect-and-quarantine.html>
}

#[cfg(target_os = "windows")]
mod windows {
    //! Internet Explorer introduced the concept of ["Security Zones"]. For our purposes, we
    //! just need to set the security zone to the "Internet" zone, which Windows will use to
    //! offer some protections.
    //!
    //! To do this, we write the [`Zone.Identifier`] NTFS alternative stream.
    //!
    //! Failure is intentionally ignored, since alternative stream are only
    //! supported by NTFS.
    //!
    //! ["Security Zones"]: https://learn.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/platform-apis/ms537183(v=vs.85)
    //! [`Zone.Identifier`]: https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/6e3f7352-d11c-4d76-8c39-2516a9df36e8
    use std::fs::OpenOptions;
    use std::io::{self, Write};
    use std::path::Path;

    /// The value 3 corresponds with the Internet Zone.
    const ZONE_IDENTIFIER_CONTENTS: &str = "[ZoneTransfer]\r\nZoneId=3";

    pub fn mark_as_downloaded(path: &Path) {
        _ = mark_as_downloaded_impl(path);
    }

    fn mark_as_downloaded_impl(path: &Path) -> io::Result<()> {
        let mut stream_path = path.to_owned();
        stream_path.as_mut_os_string().push(":Zone.Identifier");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(stream_path)?;
        write!(file, "{ZONE_IDENTIFIER_CONTENTS}")?;
        Ok(())
    }
}
