#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![feature(if_let_guard)]
#![feature(let_chains)]
#![feature(concat_idents)]

use eframe::{
    egui::{self, Layout, RichText, Ui, WidgetText},
    emath::Align,
};
use egui::Color32;
use receive::ReceiveView;
use send::SendView;
use view_switcher::{view_switcher, ViewSwitcher};

mod egui_ext;
mod error;
mod receive;
mod send;
mod tab_button;
mod utils;
mod view_switcher;

#[derive(Default)]
pub struct PortalApp {
    send_view: SendView,
    receive_view: ReceiveView,
}

#[derive(PartialEq)]
pub enum View {
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
            View::Send => matches!(self.send_view, SendView::Ready(..)),
            View::Receive => self.receive_view.show_switcher(),
        }
    }

    fn label(&self, view: &Self::View) -> RichText {
        match view {
            View::Send => RichText::new("📤 Send"),
            View::Receive => RichText::new("📥 Receive"),
        }
    }

    fn apply_style_overrides(&self, view: &Self::View, style: &mut egui::Style) {
        let (fill, stroke) = match view {
            View::Send if style.visuals.dark_mode => (from_hex(0xDB8400), from_hex(0x38270E)),
            View::Send => (from_hex(0xFF9D0A), from_hex(0x523A16)),
            View::Receive if style.visuals.dark_mode => (from_hex(0x27A7D8), from_hex(0x183039)),
            View::Receive => (from_hex(0x73CDF0), from_hex(0x183039)),
        };
        style.visuals.selection.bg_fill = fill;
        style.visuals.selection.stroke.color = stroke;
    }

    fn ui(&mut self, ui: &mut egui::Ui, view: &Self::View) {
        match view {
            View::Send => self.send_view.ui(ui),
            View::Receive => self.receive_view.ui(ui),
        }
    }
}

fn page<'a>(
    ui: &mut Ui,
    title: impl Into<RichText>,
    text: impl Into<WidgetText>,
    icon: impl Into<Option<&'a str>>,
) {
    if let Some(icon) = icon.into() {
        ui.label(RichText::new(icon).size(120.0));
    }
    ui.add_space(10.0);
    ui.label(title.into().size(30.0).strong());
    ui.add_space(10.0);
    ui.label(text);
}

fn page_with_content<'a, T>(
    ui: &mut Ui,
    title: impl Into<RichText>,
    text: impl Into<WidgetText>,
    icon: impl Into<Option<&'a str>>,
    add_contents: impl FnOnce(&mut Ui) -> T,
) -> T {
    page(ui, title, text, icon);
    ui.add_space(20.0);
    add_contents(ui)
}

#[macro_export]
macro_rules! update {
    ($target:expr, $pattern:pat => $match_arm:expr) => {
        ::take_mut::take($target, |target| match target {
            $pattern => $match_arm,
            _ => target,
        });
    };
}

fn from_hex(hex: u32) -> Color32 {
    assert!(hex < 1 << 24);
    Color32::from_rgb(
        (hex >> 16 & 0xFF) as u8,
        (hex >> 8 & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}
