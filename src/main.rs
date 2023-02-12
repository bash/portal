use eframe::egui::{self};
use portal::PortalApp;

// TODO: show version somewhere in UI
// TODO: cancellation support for sending
// TODO: distinguish primary and secondary buttons
// TODO: Confirm exit while operation in progress
// TODO: Cancellation support for uncancellable futures
#[tokio::main]
async fn main() -> Result<(), eframe::Error> {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 400.0)),
        follow_system_theme: true,
        ..Default::default()
    };
    eframe::run_native(
        "Portal",
        options,
        Box::new(|_cc| Box::<PortalApp>::default()),
    )
}
