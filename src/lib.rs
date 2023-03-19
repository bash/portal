use eframe::epaint::QuadraticBezierShape;
use eframe::Theme;
use egui::emath::Align;
use egui::widget_text::WidgetTextGalley;
use egui::{
    self, vec2, Align2, Color32, Id, Layout, Pos2, Rect, RichText, Stroke, TextStyle, Ui, Vec2,
    Visuals, WidgetText,
};
use font::{font_definitions, ICON_X};
use main_view::{show_main_view, MainViewState};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::f32::consts::TAU;
use std::vec;
use themed_visuals::ThemedVisuals;
use visuals::dark_visuals;
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
mod themed_visuals;
mod transit_info;
mod visuals;
mod widgets;

pub struct PortalApp {
    state: PortalAppState,
    visuals: ThemedVisuals,
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
    pub fn new(cc: &eframe::CreationContext, action: StartupAction) -> Self {
        // cc.egui_ctx.set_fonts(font_definitions());

        PortalApp {
            visuals: ThemedVisuals::default().set_visuals(Theme::Dark, dark_visuals()),
            state: PortalAppState::from(action),
        }
    }
}

impl eframe::App for PortalApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.visuals.apply(ctx, frame.info().system_theme);
        app_version(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.visuals_mut().selection.stroke.width = 2.;

                let theme = self.visuals.get_theme_override(ctx);

                let button_size = ui.spacing().interact_size.y * egui::vec2(2., 2.);

                let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());

                let visuals = ui
                    .style()
                    .interact_selectable(&response, matches!(theme, None));

                let dark_panel_fill = self.visuals.visuals(Theme::Dark, |v| v.panel_fill);
                let light_panel_fill = self.visuals.visuals(Theme::Light, |v| v.panel_fill);

                ui.painter()
                    .with_clip_rect(Rect::from_min_max(
                        rect.min,
                        rect.max - vec2(rect.width() / 2., 0.),
                    ))
                    .circle(
                        rect.center(),
                        button_size.x / 2.,
                        dark_panel_fill,
                        Stroke::NONE,
                    );

                ui.painter()
                    .with_clip_rect(Rect::from_min_max(
                        rect.min + vec2(rect.width() / 2., 0.),
                        rect.max,
                    ))
                    .circle(
                        rect.center(),
                        button_size.x / 2.,
                        light_panel_fill,
                        Stroke::NONE,
                    );

                ui.painter().circle(
                    rect.center(),
                    button_size.x / 2.,
                    Color32::TRANSPARENT,
                    visuals.fg_stroke,
                );

                if matches!(theme, None) {
                    paint_check(ui, button_size, rect.center());
                }

                if response.clicked() {
                    self.visuals.theme_override(ctx, None);
                }

                let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());
                let visuals = ui
                    .style()
                    .interact_selectable(&response, matches!(theme, Some(Theme::Light)));
                ui.painter().circle(
                    rect.center(),
                    button_size.x / 2.,
                    light_panel_fill,
                    visuals.fg_stroke,
                );
                if matches!(theme, Some(Theme::Light)) {
                    paint_check(ui, button_size, rect.center())
                }
                if response.clicked() {
                    self.visuals.theme_override(ctx, Theme::Light);
                }

                let (rect, response) = ui.allocate_exact_size(button_size, egui::Sense::click());
                let visuals = ui
                    .style()
                    .interact_selectable(&response, matches!(theme, Some(Theme::Dark)));
                ui.painter().circle(
                    rect.center(),
                    button_size.x / 2.,
                    dark_panel_fill,
                    visuals.fg_stroke,
                );
                if matches!(theme, Some(Theme::Dark)) {
                    paint_check(ui, button_size, rect.center())
                }
                if response.clicked() {
                    self.visuals.theme_override(ctx, Theme::Dark);
                }
            });

            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                match &mut self.state {
                    PortalAppState::Main(main) => show_main_view(main, ui),
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

fn paint_check(ui: &mut Ui, button_size: Vec2, button_center: Pos2) {
    let check_center = button_center;
    let check_radius = ui.spacing().interact_size.y / 2.;
    // let check_center = button_center + Vec2::angled(3. / 8. * TAU) * (button_size.x / 2.);

    ui.painter().circle(
        check_center,
        check_radius,
        ui.visuals().selection.bg_fill,
        Stroke::NONE,
    );

    let text = WidgetText::from(RichText::new("\u{2714}").size(check_radius * 1.5)).into_galley(
        ui,
        None,
        2. * check_radius,
        TextStyle::Button,
    );

    let text_pos = check_center - text.size() / 2.;
    text.paint_with_color_override(ui.painter(), text_pos, ui.visuals().selection.stroke.color);
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ThemePreference {
    /// Dark mode: light text on a dark background.
    Dark,

    /// Light mode: dark text on a light background.
    Light,

    /// Follow the system's theme preference.
    System,
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
