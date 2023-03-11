use crate::font::{ICON_ARROW_LEFT, ICON_X};
use egui::{Key, Modifiers, Ui};
use std::fmt;

pub fn cancel_button(ui: &mut Ui, label: CancelLabel) -> bool {
    ui.horizontal(|ui| ui.button(format!("{label}")).clicked())
        .inner
        || ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
}

pub enum CancelLabel {
    Cancel,
    Back,
}

impl fmt::Display for CancelLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CancelLabel::Cancel => write!(f, "{ICON_X} Cancel"),
            CancelLabel::Back => write!(f, "{ICON_ARROW_LEFT} Back"),
        }
    }
}
