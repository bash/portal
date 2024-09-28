#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use egui::{vec2, IconData, ViewportBuilder};
use portal::{PortalApp, StartupAction};
use std::error::Error;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(last = true)]
    uri: Option<String>,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let mut viewport = ViewportBuilder::default().with_inner_size(vec2(320.0, 500.0));
    if let Some(icon) = icon()? {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        run_and_return: false,
        ..Default::default()
    };
    let startup_action = StartupAction::from_uri(args.uri.as_deref());
    eframe::run_native(
        "Portal",
        options,
        Box::new(move |cc| Ok(Box::new(PortalApp::new(cc, startup_action)))),
    )?;
    Ok(())
}

#[cfg(not(any(windows, all(debug_assertions, target_os = "macos"))))]
fn icon() -> Result<Option<IconData>, Box<dyn Error>> {
    Ok(None)
}

#[cfg(all(debug_assertions, target_os = "macos"))]
fn icon() -> Result<Option<IconData>, Box<dyn Error>> {
    eframe::icon_data::from_png_bytes(include_bytes!(
        "../build/macos/AppIcon.iconset/icon_256x256@2x.png"
    ))
    .map(Some)
    .map_err(Into::into)
}

#[cfg(windows)]
fn icon() -> Result<Option<IconData>, Box<dyn Error>> {
    eframe::icon_data::from_png_bytes(include_bytes!("../build/windows/icon-256x256.png"))
        .map(Some)
        .map_err(Into::into)
}
