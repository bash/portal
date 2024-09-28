//! An egui plugin that syncs egui's current theme to the viewport's.

use egui::{Context, Id, SystemTheme, ThemePreference, ViewportCommand};
use std::sync::Arc;

// We use Id::NULL because there's only one instance of this plugin.
const PLUGIN_ID: Id = Id::NULL;

pub(crate) fn register(ctx: &Context) {
    if ctx.data(|d| d.get_temp::<State>(PLUGIN_ID).is_none()) {
        ctx.on_end_pass("update_viewport_theme", Arc::new(State::end_pass));
    }
}

#[derive(Debug, Clone)]
struct State(ThemePreference);

impl State {
    fn end_pass(ctx: &Context) {
        let preference = ctx.options(|opt| opt.theme_preference);
        let has_changed = !ctx
            .data(|d| d.get_temp::<State>(PLUGIN_ID))
            .is_some_and(|old| old.0 == preference);
        if has_changed {
            ctx.send_viewport_cmd(ViewportCommand::SetTheme(to_system_theme(preference)));
            ctx.data_mut(|d| d.insert_temp(PLUGIN_ID, State(preference)));
        }
    }
}

fn to_system_theme(preference: ThemePreference) -> SystemTheme {
    match preference {
        ThemePreference::System => SystemTheme::SystemDefault,
        ThemePreference::Dark => SystemTheme::Dark,
        ThemePreference::Light => SystemTheme::Light,
    }
}
