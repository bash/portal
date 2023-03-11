use eframe::Theme;
use egui::epaint::hex_color;
use egui::{Context, Visuals};

pub(crate) struct CustomVisuals {
    current_theme: Option<Theme>,
    dark: Visuals,
    light: Visuals,
}

impl CustomVisuals {
    pub(crate) fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        let theme = frame.info().system_theme.unwrap_or(Theme::Light);
        if self.current_theme != Some(theme) {
            ctx.set_visuals(self.visuals(theme).clone());
            self.current_theme = Some(theme);
        }
    }

    fn visuals(&self, theme: Theme) -> &Visuals {
        match theme {
            Theme::Dark => &self.dark,
            Theme::Light => &self.light,
        }
    }
}

impl Default for CustomVisuals {
    fn default() -> Self {
        Self {
            current_theme: None,
            dark: dark_visuals(),
            light: Visuals::light(),
        }
    }
}

fn dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = hex_color!("#121212");
    visuals.widgets.inactive.weak_bg_fill = hex_color!("#292929");
    visuals
}
