use egui::{Button, Response, Ui, Vec2, Widget, WidgetText};

pub struct PrimaryButton {
    text: WidgetText,
    min_size: Vec2,
}

impl PrimaryButton {
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            min_size: Vec2::ZERO,
        }
    }

    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }
}

impl Widget for PrimaryButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let fill_color = ui.style().visuals.selection.bg_fill;
        let text_color = ui.style().visuals.selection.stroke.color;
        Button::new(self.text.color(text_color))
            .fill(fill_color)
            .min_size(self.min_size)
            .ui(ui)
    }
}
