#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{
    egui::{self, Layout, RichText, Ui, WidgetText},
    emath::Align,
};
use receive::ReceiveView;
use send::SendView;
use view_switcher::{view_switcher, ViewSwitcher};

mod receive;
mod send;
mod tab_button;
mod view_switcher;

// TODO: show version somewhere in UI
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
    receive_view: ReceiveView,
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

    fn show_switcher(&self, view: &Self::View) -> bool {
        match view {
            View::Send => matches!(self.send_view, SendView::Ready),
            View::Receive => true,
        }
    }

    fn label(&self, view: &Self::View) -> RichText {
        match view {
            View::Send => RichText::new("📤 Send"),
            View::Receive => RichText::new("📥 Receive"),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, view: &Self::View) {
        match view {
            View::Send => self.send_view.ui(ui),
            View::Receive => self.receive_view.ui(ui),
        }
    }
}

pub fn page(
    ui: &mut Ui,
    title: impl Into<RichText>,
    text: impl Into<WidgetText>,
    icon: impl Into<RichText>,
) {
    ui.label(icon.into().size(120.0));
    ui.add_space(10.0);
    ui.label(title.into().size(30.0).strong());
    ui.add_space(10.0);
    ui.label(text);
}

pub fn page_with_content(
    ui: &mut Ui,
    title: impl Into<RichText>,
    text: impl Into<WidgetText>,
    icon: impl Into<RichText>,
    add_contents: impl FnOnce(&mut Ui),
) {
    page(ui, title, text, icon);
    ui.add_space(20.0);
    add_contents(ui);
}
