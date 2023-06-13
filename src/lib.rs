#![warn(clippy::str_to_string)]
#![warn(clippy::unwrap_used)]

use eframe::Theme;
use egui::emath::Align;
use egui::{self, Layout, Ui};
use egui_ext::ContextExt;
use font::{font_definitions, ICON_X};
use main_view::{show_main_view, MainViewState};
use poll_promise::Promise;
use std::error::Error;
use version::{get_or_update_latest_app_version, AppVersion};
use visuals::{Accent, CustomVisuals};
use widgets::{app_menu, cancel_button, page, CancelLabel};

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
mod version;
mod visuals;
mod widgets;

pub struct PortalApp {
    state: PortalAppState,
    visuals: CustomVisuals,
    version: Promise<Option<AppVersion>>,
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
            version: cc
                .egui_ctx
                .spawn_async(get_or_update_latest_app_version(cc.egui_ctx.clone())),
        }
    }
}

impl eframe::App for PortalApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.visuals.update(self.accent(), ctx, frame);
        app_menu(ctx, self.version.ready().cloned().flatten());

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

impl PortalApp {
    fn accent(&self) -> Accent {
        match &self.state {
            PortalAppState::Main(m) => m.accent(),
            PortalAppState::UriError(_) => Accent::Orange,
        }
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
        ::replace_with::replace_with($target, Default::default, |target| match target {
            $pattern => $match_arm,
            _ => target,
        });
    };
}
