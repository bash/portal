use egui::epaint::hex_color;
use egui::{Style, Theme};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Accent {
    Orange,
    Blue,
}

pub(crate) fn apply_accent(theme: Theme, accent: Accent) -> impl FnOnce(&mut Style) {
    move |style| {
        let (fill, stroke) = match (accent, theme) {
            (Accent::Orange, Theme::Dark) => (hex_color!("#DB8400"), hex_color!("#38270E")),
            (Accent::Orange, Theme::Light) => (hex_color!("#FF9D0A"), hex_color!("#523A16")),
            (Accent::Blue, Theme::Dark) => (hex_color!("#27A7D8"), hex_color!("#183039")),
            (Accent::Blue, Theme::Light) => (hex_color!("#73CDF0"), hex_color!("#183039")),
        };
        style.visuals.selection.bg_fill = fill;
        style.visuals.selection.stroke.color = stroke;
        style.visuals.hyperlink_color = fill;
    }
}
