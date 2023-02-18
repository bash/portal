use egui::{RichText, Ui, WidgetText};

pub fn page<'a>(
    ui: &mut Ui,
    title: impl Into<RichText>,
    text: impl Into<WidgetText>,
    icon: impl Into<Option<&'a str>>,
) {
    if let Some(icon) = icon.into() {
        ui.add_space(10.);
        ui.label(RichText::new(icon).size(120.0));
    }
    ui.add_space(10.0);
    ui.label(title.into().size(30.0).strong());
    ui.add_space(10.0);
    ui.label(text);
}

pub fn page_with_content<'a, T>(
    ui: &mut Ui,
    title: impl Into<RichText>,
    text: impl Into<WidgetText>,
    icon: impl Into<Option<&'a str>>,
    add_contents: impl FnOnce(&mut Ui) -> T,
) -> T {
    page(ui, title, text, icon);
    ui.add_space(20.0);
    add_contents(ui)
}
