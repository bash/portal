use egui::{FontData, FontDefinitions, FontFamily, FontTweak};

pub const ICON_UPLOAD: char = '\u{f452}';
pub const ICON_DOWNLOAD: char = '\u{f215}';
pub const ICON_CLIPBOARD_COPY: char = '\u{f1b8}';
pub const ICON_TICKET: char = '\u{f431}';
pub const ICON_X: char = '\u{f47d}';
pub const ICON_ARROW_LEFT: char = '\u{f137}';
pub const ICON_CHECK: char = '\u{f198}';

const LUCIDE_FONT_NAME: &str = "lucide";
const ROBOTO_FONT_NAME: &str = "Roboto";

pub fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        ROBOTO_FONT_NAME.to_owned(),
        FontData::from_static(include_bytes!("../assets/roboto/Roboto-Regular.ttf")),
    );
    fonts.font_data.insert(
        LUCIDE_FONT_NAME.to_owned(),
        FontData::from_static(include_bytes!("../assets/lucide/lucide.ttf")).tweak(FontTweak {
            y_offset_factor: -0.05,
            ..Default::default()
        }),
    );
    fonts.families.insert(
        FontFamily::Proportional,
        vec![ROBOTO_FONT_NAME.to_owned(), LUCIDE_FONT_NAME.to_owned()],
    );
    fonts
}
