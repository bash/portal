use ico_builder::IcoBuilder;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    match std::env::args().nth(1).as_deref() {
        Some("update-ico") => update_ico(),
        Some(subcommand) => Err(format!("Unknown subcommand '{subcommand}'").into()),
        None => Err("Missing subcommand".into()),
    }
}

fn update_ico() -> Result<(), Box<dyn Error>> {
    IcoBuilder::default()
        .add_source_file("build/windows/icon-32x32.png")
        .add_source_file("build/windows/icon-256x256.png")
        .build_file("build/windows/portal.ico")
        .map_err(Into::into)
}
