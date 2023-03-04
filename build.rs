use std::error::Error;
use windows_app_icon::cargo::build_ico_file;
use windows_app_icon::IconSizes;
use winresource::WindowsResource;

fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(windows)]
    build_windows()?;
    Ok(())
}

#[cfg(windows)]
fn build_windows() -> Result<(), Box<dyn Error>> {
    let icon = build_ico_file(
        "windows-app-icon.ico",
        [
            "build/windows/icon-32x32.png",
            "build/windows/icon-1920x1920.png",
        ],
        IconSizes::MINIMAL,
    )?;
    let mut res = WindowsResource::new();
    res.set_icon(icon.to_str().unwrap());
    res.compile()?;
    Ok(())
}
