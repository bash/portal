use eframe::egui::{Id, RichText, Ui};

use crate::tab_button::TabButton;

pub trait ViewSwitcher {
    type View;

    fn show_switcher(&self, view: &Self::View) -> bool;

    fn label(&self, view: &Self::View) -> RichText;

    fn ui(&mut self, ui: &mut Ui, view: &Self::View);
}

pub fn view_switcher<S, V>(ui: &mut Ui, id: Id, views: &[V], switcher: &mut S)
where
    S: ViewSwitcher<View = V>,
    V: PartialEq,
{
    let active_view_index = ui.memory().data.get_persisted::<usize>(id).unwrap_or(0);

    if switcher.show_switcher(&views[active_view_index]) {
        ui.horizontal(|ui| {
            for (index, view) in views.iter().enumerate() {
                let is_active = active_view_index == index;
                let button = TabButton::new(switcher.label(view)).selected(is_active);
                if ui.add(button).clicked() {
                    ui.memory().data.insert_persisted(id, index);
                }
            }
        });
    }

    for (index, view) in views.iter().enumerate() {
        let is_active = active_view_index == index;
        if is_active {
            switcher.ui(ui, view);
        }
    }
}
