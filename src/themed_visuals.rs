use eframe::{Frame, Theme};
use egui::{trace, Context, Id, Ui, Visuals};

#[derive(Clone, Debug, PartialEq)]
pub struct ThemedVisuals {
    current_theme: Option<Theme>,
    default_theme: Theme,
    dark: Visuals,
    light: Visuals,
}

impl Default for ThemedVisuals {
    fn default() -> Self {
        ThemedVisuals {
            current_theme: None,
            default_theme: Theme::Dark,
            dark: Visuals::dark(),
            light: Visuals::light(),
        }
    }
}

impl ThemedVisuals {
    pub fn apply(&mut self, ctx: &Context, system_theme: Option<Theme>) {
        let theme = self.get_theme(ctx, system_theme);
        if self.current_theme != Some(theme) {
            ctx.set_visuals(self.get_visuals(theme));
            self.current_theme = Some(theme);
        }
    }

    pub fn theme_override(&mut self, ctx: &Context, theme: impl Into<Option<Theme>>) {
        ctx.memory_mut(|memory| match theme.into() {
            None => memory.data.remove::<Theme>(theme_override_id()),
            Some(theme) => memory.data.insert_persisted(theme_override_id(), theme),
        });
    }

    pub fn get_theme_override(&self, ctx: &Context) -> Option<Theme> {
        ctx.memory_mut(|memory| memory.data.get_persisted(theme_override_id()))
    }

    // TODO: move to builder
    pub fn default_theme(mut self, theme: Theme) -> Self {
        self.default_theme = theme;
        self
    }

    // TODO: move to builder
    pub fn set_visuals(mut self, theme: Theme, visuals: Visuals) -> Self {
        match theme {
            Theme::Dark => self.dark = visuals,
            Theme::Light => self.light = visuals,
        }
        self
    }

    pub fn get_visuals(&self, theme: Theme) -> Visuals {
        match theme {
            Theme::Dark => self.dark.clone(),
            Theme::Light => self.light.clone(),
        }
    }

    pub fn visuals<T>(&self, theme: Theme, reader: impl FnOnce(&Visuals) -> T) -> T {
        match theme {
            Theme::Dark => reader(&self.dark),
            Theme::Light => reader(&self.light),
        }
    }

    fn get_theme(&self, ctx: &Context, system_theme: Option<Theme>) -> Theme {
        self.get_theme_override(ctx)
            .or_else(|| system_theme)
            .unwrap_or(self.default_theme)
    }
}

fn theme_override_id() -> Id {
    Id::new("theme-override")
}
