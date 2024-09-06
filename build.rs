use std::error::Error;
fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(windows)]
    build_windows()?;
    Ok(())
}

#[cfg(windows)]
fn build_windows() -> Result<(), Box<dyn Error>> {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("build/windows/portal.ico");
    res.compile()?;
    Ok(())
}
