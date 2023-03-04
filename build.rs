use std::error::Error;
fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(windows)]
    build_windows()?;
    Ok(())
}

#[cfg(windows)]
fn build_windows() -> Result<(), Box<dyn Error>> {
    use std::fs;
    use std::path::PathBuf;

    use ico_builder::IcoBuilder;
    use winresource::WindowsResource;

    let icon = IcoBuilder::default()
        .add_source_file("build/windows/icon-32x32.png")
        .add_source_file("build/windows/icon-256x256.png")
        .build_file_cargo("windows-app-icon.ico")?;

    let mut ico_file_path = PathBuf::from("target");
    ico_file_path.push("icon.ico");
    fs::copy(&icon, ico_file_path)?;

    let mut res = WindowsResource::new();
    res.set_icon(icon.to_str().unwrap());
    res.compile()?;
    Ok(())
}
