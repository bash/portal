use eframe::Theme;
use egui::emath::Align;
use egui::{self, Layout, Ui};
use font::{font_definitions, ICON_X};
use main_view::{show_main_view, MainViewState};
use std::error::Error;
use visuals::CustomVisuals;
use widgets::{app_version, cancel_button, page, CancelLabel};

mod egui_ext;
mod font;
mod receive;
pub(crate) use receive::*;
mod send;
pub(crate) use send::*;
mod shell;
mod startup_action;
pub use startup_action::*;
mod main_view;
mod transit_info;
mod visuals;
mod widgets;

pub struct PortalApp {
    state: PortalAppState,
    visuals: CustomVisuals,
}

enum PortalAppState {
    Main(MainViewState),
    UriError(Box<dyn Error>),
}

impl Default for PortalAppState {
    fn default() -> Self {
        PortalAppState::Main(Default::default())
    }
}

impl From<StartupAction> for PortalAppState {
    fn from(value: StartupAction) -> Self {
        match value {
            StartupAction::ShowInvalidUriError(error) => PortalAppState::UriError(error),
            StartupAction::None => Default::default(),
            StartupAction::ReceiveFile(action) => PortalAppState::Main(MainViewState::from(action)),
        }
    }
}

impl PortalApp {
    pub fn new(cc: &eframe::CreationContext, action: StartupAction, default_theme: Theme) -> Self {
        cc.egui_ctx.set_fonts(font_definitions());

        PortalApp {
            visuals: CustomVisuals::new(default_theme),
            state: PortalAppState::from(action),
        }
    }
}

impl eframe::App for PortalApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.visuals.update(ctx, frame);
        app_version(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                match &mut self.state {
                    PortalAppState::Main(main) => show_main_view(main, ui, frame),
                    PortalAppState::UriError(error) => {
                        if show_uri_error(ui, error.as_ref()) {
                            update!(
                                &mut self.state,
                                PortalAppState::UriError(..) => PortalAppState::default());
                        }
                    }
                }
            });
        });
    }
}

fn show_uri_error(ui: &mut Ui, error: &dyn Error) -> bool {
    let back_button_clicked = cancel_button(ui, CancelLabel::Back);
    page(ui, "Failed to open Link", error.to_string(), ICON_X);
    back_button_clicked
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
