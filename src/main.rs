#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{
    egui::{self, Layout, RichText},
    emath::Align,
};
use send::SendView;
use view_switcher::{view_switcher, ViewSwitcher};

mod send;
mod tab_button;
mod view_switcher;

#[tokio::main]
async fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 400.0)),
        follow_system_theme: true,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "Portal",
        options,
        Box::new(|_cc| Box::new(PortalApp::default())),
    )
}

#[derive(Default)]
struct PortalApp {
    send_view: SendView,
}

#[derive(PartialEq)]
enum View {
    Send,
    Receive,
}

impl eframe::App for PortalApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                view_switcher(ui, ui.next_auto_id(), &[View::Send, View::Receive], self);
            });
        });
    }
}

impl ViewSwitcher for PortalApp {
    type View = View;

    fn allow_switching(&self, _view: &Self::View) -> bool {
        matches!(self.send_view, SendView::Ready)
    }

    fn label(&self, view: &Self::View) -> RichText {
        match view {
            View::Send => RichText::new("📤 Send"),
            View::Receive => RichText::new("📥 Receive"),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, view: &Self::View) {
        if let View::Send = view {
            self.send_view.ui(ui);
        }
    }
}
