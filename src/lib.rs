use egui::emath::Align;
use egui::epaint::hex_color;
use egui::{self, Layout, RichText};
use font::{font_definitions, ICON_DOWNLOAD, ICON_UPLOAD};
use receive::ReceiveView;
use send::SendView;
use visuals::CustomVisuals;
use widgets::toggle;

mod egui_ext;
mod font;
mod receive;
mod send;
mod shell;
mod transit_info;
mod visuals;
mod widgets;

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

impl PortalApp {
    pub fn new_boxed(cc: &eframe::CreationContext) -> Box<dyn eframe::App> {
        Box::new(Self::new(cc))
    }

    fn new(cc: &eframe::CreationContext) -> Self {
        cc.egui_ctx.set_fonts(font_definitions());

        PortalApp {
            send_view: Default::default(),
            receive_view: Default::default(),
            visuals: Default::default(),
            view_toggle: Default::default(),
        }
    }
}

impl eframe::App for PortalApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.visuals.update(ctx, frame);
        show_version(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                let view = View::from(self.view_toggle);
                self.apply_style_overrides(view, ui.style_mut());

                if self.show_switcher(view) {
                    let font_size = 14.;
                    ui.add_space(12.);
                    ui.add(toggle(
                        &mut self.view_toggle,
                        RichText::new(format!("{ICON_UPLOAD} Send")).size(font_size),
                        RichText::new(format!("{ICON_DOWNLOAD} Receive")).size(font_size),
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
                ui.visuals_mut().hyperlink_color = ui.visuals().weak_text_color();

                const REPOSITORY_URL: &str = "https://github.com/bash/portal";

                #[cfg(debug_assertions)]
                ui.hyperlink_to("dev", REPOSITORY_URL);
                #[cfg(not(debug_assertions))]
                ui.hyperlink_to(
                    env!("CARGO_PKG_VERSION"),
                    format!(
                        "{REPOSITORY_URL}/releases/tag/v{}",
                        env!("CARGO_PKG_VERSION")
                    ),
                );
            })
        });
}

impl PortalApp {
    fn apply_style_overrides(&self, view: View, style: &mut egui::Style) {
        let (fill, stroke) = match view {
            View::Send if style.visuals.dark_mode => (hex_color!("#DB8400"), hex_color!("#38270E")),
            View::Send => (hex_color!("#FF9D0A"), hex_color!("#523A16")),
            View::Receive if style.visuals.dark_mode => {
                (hex_color!("#27A7D8"), hex_color!("#183039"))
            }
            View::Receive => (hex_color!("#73CDF0"), hex_color!("#183039")),
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
