use eframe::egui::{RichText, Ui};

#[derive(Default)]
pub struct ReceiveView;

impl ReceiveView {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label(RichText::new("📥").size(100.0).strong());
        ui.label(RichText::new("Receive File").size(30.0).strong());
    }
}
