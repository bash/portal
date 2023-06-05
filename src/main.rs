#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use eframe::{egui, Theme};
use portal::{PortalApp, StartupAction};
use std::error::Error;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(last = true)]
    uri: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();

    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let default_theme = default_theme()?;
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 500.0)),
        follow_system_theme: true,
        default_theme,
        run_and_return: false,
        ..Default::default()
    };
    let startup_action = StartupAction::from_uri(args.uri.as_deref());
    eframe::run_native(
        "Portal",
        options,
        Box::new(move |cc| Box::new(PortalApp::new(cc, startup_action, default_theme))),
    )?;
    Ok(())
}

/// Eframe doesn't follow the system theme on Linux.
/// See: <https://github.com/rust-windowing/winit/issues/1549>
#[cfg(target_os = "linux")]
fn default_theme() -> Result<Theme, Box<dyn Error>> {
    match dark_light::detect() {
        dark_light::Mode::Dark => Ok(Theme::Dark),
        dark_light::Mode::Light | dark_light::Mode::Default => Ok(Theme::Light),
    }
}

#[cfg(not(target_os = "linux"))]
fn default_theme() -> Result<Theme, Box<dyn Error>> {
    Ok(Theme::Light)
}
