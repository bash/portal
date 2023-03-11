use egui::{RichText, Ui, WidgetText};

pub fn page<I>(
    ui: &mut Ui,
    title: impl Into<RichText>,
    text: impl Into<WidgetText>,
    icon: impl Into<Option<I>>,
) where
    I: Into<String>,
{
    if let Some(icon) = icon.into() {
        ui.add_space(10.);
        ui.label(RichText::new(icon).size(120.0));
    }
    ui.add_space(10.0);
    ui.label(title.into().size(30.0).strong());
    ui.add_space(10.0);
    ui.label(text);
}

pub fn page_with_content<T, I>(
    ui: &mut Ui,
    title: impl Into<RichText>,
    text: impl Into<WidgetText>,
    icon: impl Into<Option<I>>,
    add_contents: impl FnOnce(&mut Ui) -> T,
) -> T
where
    I: Into<String>,
{
    page(ui, title, text, icon);
    ui.add_space(20.0);
    add_contents(ui)
}
