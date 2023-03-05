use egui::emath::Align;
use egui::{self, Color32, Layout, RichText};
use receive::ReceiveView;
use send::SendView;
use visuals::CustomVisuals;
use widgets::toggle;

mod egui_ext;
mod receive;
mod send;
mod shell;
mod transit_info;
mod visuals;
mod widgets;

#[derive(Default)]
pub struct PortalApp {
    send_view: SendView,
    receive_view: ReceiveView,
    visuals: CustomVisuals,
    view_toggle: bool,
}

#[derive(PartialEq, Copy, Clone)]
pub enum View {
    Send,
    Receive,
}

impl From<bool> for View {
    fn from(value: bool) -> Self {
        if value {
            View::Receive
        } else {
            View::Send
        }
    }
}

impl eframe::App for PortalApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.visuals.update(ctx, frame);
        show_version(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                ui.add_space(12.);

                let view = View::from(self.view_toggle);
                self.apply_style_overrides(view, ui.style_mut());

                if self.show_switcher(view) {
                    let font_size = 14.;
                    ui.add(toggle(
                        &mut self.view_toggle,
                        RichText::new("📤 Send").size(font_size),
                        RichText::new("📥 Receive").size(font_size),
                    ));
                }

                self.ui(view, ui);
            });
        });
    }
}

fn show_version(ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("bottom panel")
        .show_separator_line(false)
        .show(ctx, |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                #[cfg(debug_assertions)]
                ui.weak("dev");
                #[cfg(not(debug_assertions))]
                ui.weak(env!("CARGO_PKG_VERSION"));
            })
        });
}

impl PortalApp {
    fn apply_style_overrides(&self, view: View, style: &mut egui::Style) {
        let (fill, stroke) = match view {
            View::Send if style.visuals.dark_mode => (from_hex(0xDB8400), from_hex(0x38270E)),
            View::Send => (from_hex(0xFF9D0A), from_hex(0x523A16)),
            View::Receive if style.visuals.dark_mode => (from_hex(0x27A7D8), from_hex(0x183039)),
            View::Receive => (from_hex(0x73CDF0), from_hex(0x183039)),
        };
        style.visuals.selection.bg_fill = fill;
        style.visuals.selection.stroke.color = stroke;
    }

    fn show_switcher(&self, view: View) -> bool {
        match view {
            View::Send => matches!(self.send_view, SendView::Ready(..)),
            View::Receive => self.receive_view.show_switcher(),
        }
    }

    fn ui(&mut self, view: View, ui: &mut egui::Ui) {
        match view {
            View::Send => self.send_view.ui(ui),
            View::Receive => self.receive_view.ui(ui),
        }
    }
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
