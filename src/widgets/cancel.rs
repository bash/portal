use egui::{Key, Modifiers, Ui};

pub fn cancel_button(ui: &mut Ui, label: CancelLabel) -> bool {
    ui.horizontal(|ui| ui.button(label.as_str()).clicked())
        .inner
        || ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
}

pub enum CancelLabel {
    Cancel,
    Back,
}

impl CancelLabel {
    fn as_str(&self) -> &str {
        match self {
            CancelLabel::Cancel => "Cancel",
            CancelLabel::Back => "Back",
        }
    }
}
