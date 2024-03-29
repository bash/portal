use eframe::Theme;
use egui::epaint::hex_color;
use egui::{Context, Visuals};
use log::trace;

#[derive(Debug)]
pub(crate) struct CustomVisuals {
    current: Option<(Theme, Accent)>,
    default_theme: Theme,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Accent {
    Orange,
    Blue,
}

impl CustomVisuals {
    pub(crate) fn new(default_theme: Theme) -> Self {
        Self {
            current: None,
            default_theme,
        }
    }

    pub(crate) fn update(&mut self, accent: Accent, ctx: &Context, frame: &mut eframe::Frame) {
        let theme = frame.info().system_theme.unwrap_or(self.default_theme);
        if self.current != Some((theme, accent)) {
            ctx.set_visuals(visuals(theme, accent));
            trace!("Updating visuals for theme {theme:?} and accent {accent:?}");
            self.current = Some((theme, accent));
        }
    }
}

fn visuals(theme: Theme, accent: Accent) -> Visuals {
    let mut visuals = match theme {
        Theme::Dark => dark_visuals(),
        Theme::Light => Visuals::light(),
    };
    apply_accent(&mut visuals, theme, accent);
    visuals
}

fn dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = hex_color!("#121212");
    visuals.widgets.inactive.weak_bg_fill = hex_color!("#292929");
    visuals
}

fn apply_accent(visuals: &mut Visuals, theme: Theme, accent: Accent) {
    let (fill, stroke) = match (accent, theme) {
        (Accent::Orange, Theme::Dark) => (hex_color!("#DB8400"), hex_color!("#38270E")),
        (Accent::Orange, Theme::Light) => (hex_color!("#FF9D0A"), hex_color!("#523A16")),
        (Accent::Blue, Theme::Dark) => (hex_color!("#27A7D8"), hex_color!("#183039")),
        (Accent::Blue, Theme::Light) => (hex_color!("#73CDF0"), hex_color!("#183039")),
    };
    visuals.selection.bg_fill = fill;
    visuals.selection.stroke.color = stroke;
    visuals.hyperlink_color = fill;
}
