#[cfg(not(windows))]
pub fn open(path: &std::path::Path) -> Result<(), opener::OpenError> {
    opener::open(path)
}

#[cfg(windows)]
pub use self::windows::*;

// Mostly copied from <https://github.com/Seeker14491/opener/blob/master/opener/src/windows.rs>
// but changed to use ShellExecuteExW instead of ShellExecuteW.
// I'm intentionally using ShellExecuteExW instead of ShellExecuteW because ShellExecuteW does not show
// SmartScreen warnings. I think these warnings are stupid, but I want to have the same behaviour as
// when you open a file downloaded via your browser.
#[cfg(windows)]
mod windows {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::{io, ptr};
    use winapi::ctypes::c_int;
    use winapi::um::shellapi::{ShellExecuteExW, SHELLEXECUTEINFOW};

    pub fn open(path: &Path) -> io::Result<()> {
        const SW_SHOW: c_int = 5;

        let path = convert_path(path.as_os_str())?;
        let operation: Vec<u16> = OsStr::new("open\0").encode_wide().collect();
        let result = unsafe {
            let mut info = SHELLEXECUTEINFOW {
                cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
                fMask: 0,
                hwnd: ptr::null_mut(),
                lpVerb: operation.as_ptr(),
                lpFile: path.as_ptr(),
                lpParameters: ptr::null_mut(),
                lpDirectory: ptr::null_mut(),
                nShow: SW_SHOW,
                hInstApp: ptr::null_mut(),
                lpIDList: ptr::null_mut(),
                lpClass: ptr::null_mut(),
                hkeyClass: ptr::null_mut(),
                dwHotKey: 0,
                hMonitor: ptr::null_mut(),
                hProcess: ptr::null_mut(),
            };
            ShellExecuteExW(&mut info)
        };
        if result as c_int > 32 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn convert_path(path: &OsStr) -> io::Result<Vec<u16>> {
        let mut maybe_result: Vec<u16> = path.encode_wide().collect();
        if maybe_result.iter().any(|&u| u == 0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "path contains NUL byte(s)",
            ));
        }

        maybe_result.push(0);
        Ok(maybe_result)
    }
}
