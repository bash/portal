use eframe::egui::{Id, RichText, Ui};
use egui::Style;

use crate::tab_button::TabButton;

pub trait ViewSwitcher {
    type View;

    fn show_switcher(&self, view: &Self::View) -> bool;

    fn label(&self, view: &Self::View) -> RichText;

    fn apply_style_overrides(&self, view: &Self::View, style: &mut Style);

    fn ui(&mut self, ui: &mut Ui, view: &Self::View);
}

pub fn view_switcher<S, V>(ui: &mut Ui, id: Id, views: &[V], switcher: &mut S)
where
    S: ViewSwitcher<View = V>,
    V: PartialEq,
{
    let active_view_index = ui
        .memory_mut(|memory| memory.data.get_persisted::<usize>(id))
        .unwrap_or(0);

    if switcher.show_switcher(&views[active_view_index]) {
        ui.horizontal(|ui| {
            for (index, view) in views.iter().enumerate() {
                let is_active = active_view_index == index;
                let button = TabButton::new(switcher.label(view)).selected(is_active);

                if is_active {
                    switcher.apply_style_overrides(view, ui.style_mut());
                }

                if ui.add(button).clicked() {
                    ui.memory_mut(|memory| memory.data.insert_persisted(id, index));
                }
            }
        });
    }

    for (index, view) in views.iter().enumerate() {
        let is_active = active_view_index == index;
        if is_active {
            switcher.apply_style_overrides(view, ui.style_mut());
            switcher.ui(ui, view);
        }
    }
}
