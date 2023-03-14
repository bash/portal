#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use eframe::egui::{self};
use portal::{PortalApp, StartupAction};

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(last = true)]
    uri: Option<String>,
}

fn main() -> Result<(), eframe::Error> {
    let args = Cli::parse();

    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 500.0)),
        follow_system_theme: true,
        run_and_return: false,
        ..Default::default()
    };
    let startup_action = StartupAction::from_uri(args.uri.as_deref());
    eframe::run_native(
        "Portal",
        options,
        Box::new(move |cc| Box::new(PortalApp::new(cc, startup_action))),
    )
}
