use eframe::egui::{Id, RichText, Ui};

use crate::tab_button::TabButton;

pub trait ViewSwitcher {
    type View;

    fn allow_switching(&self, view: &Self::View) -> bool;

    fn label(&self, view: &Self::View) -> RichText;

    fn ui(&mut self, ui: &mut Ui, view: &Self::View);
}

pub fn view_switcher<S, V>(ui: &mut Ui, id: Id, views: &[V], switcher: &mut S)
where
    S: ViewSwitcher<View = V>,
    V: PartialEq,
{
    let buttons_enabled = views.iter().all(|v| switcher.allow_switching(v));
    let active_view_index = ui.memory().data.get_persisted::<usize>(id).unwrap_or(0);

    ui.horizontal(|ui| {
        for (index, view) in views.iter().enumerate() {
            let is_active = active_view_index == index;
            if ui
                .add(
                    TabButton::new(switcher.label(view))
                        .enabled(buttons_enabled)
                        .selected(is_active),
                )
                .clicked()
            {
                ui.memory().data.insert_persisted(id, index);
            }
        }
    });

    for (index, view) in views.iter().enumerate() {
        let is_active = active_view_index == index;
        if is_active {
            switcher.ui(ui, view);
        }
    }
}
