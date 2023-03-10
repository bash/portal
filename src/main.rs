#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self};
use portal::PortalApp;

fn main() -> Result<(), eframe::Error> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 500.0)),
        follow_system_theme: true,
        run_and_return: false,
        ..Default::default()
    };
    eframe::run_native("Portal", options, Box::new(PortalApp::new_boxed))
}
