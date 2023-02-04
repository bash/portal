use eframe::egui::{Response, RichText, Ui, Widget};
use eframe::epaint::Vec2;

pub struct TabButton {
    text: RichText,
    selected: bool,
}

impl TabButton {
    pub fn new(text: impl Into<RichText>) -> Self {
        TabButton {
            text: text.into().size(14.0),
            selected: false,
        }
    }

    pub fn selected(self, selected: bool) -> Self {
        TabButton { selected, ..self }
    }
}

impl Widget for TabButton {
    fn ui(self, ui: &mut Ui) -> Response {
        ui.scope(|ui| {
            ui.style_mut().spacing.button_padding = Vec2::new(10.0, 8.0);
            ui.selectable_label(self.selected, self.text)
        })
        .inner
    }
}
