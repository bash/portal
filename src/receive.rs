use eframe::egui::Ui;

#[derive(Default)]
pub struct ReceiveView;

impl ReceiveView {
    pub fn ui(&mut self, ui: &mut Ui) {
        crate::page(ui, "Receive File", "Lorem Ipsum", "📥");
    }
}
