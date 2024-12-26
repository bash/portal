use egui::{FontData, FontDefinitions, FontFamily, FontTweak};
use std::sync::Arc;

pub const ICON_UPLOAD: char = '\u{f539}';
pub const ICON_DOWNLOAD: char = '\u{f27b}';
pub const ICON_CLIPBOARD_COPY: char = '\u{f211}';
pub const ICON_TICKET: char = '\u{f511}';
pub const ICON_X: char = '\u{f573}';
pub const ICON_ARROW_LEFT: char = '\u{f14c}';
pub const ICON_CHECK: char = '\u{f1e5}';
pub const ICON_LINK: char = '\u{f380}';
pub const ICON_REFRESH_CW: char = '\u{f464}';
pub const ICON_TAG: char = '\u{f4fe}';

const LUCIDE_FONT_NAME: &str = "lucide";
const INTER_MEDIUM: &str = "Inter Medium";
const INTER_BOLD: &str = "Inter Bold";

pub fn title_font_family() -> FontFamily {
    FontFamily::Name(Arc::from("Title"))
}

pub fn font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        INTER_MEDIUM.to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/inter/Inter-Medium.otf"
        ))),
    );
    fonts.font_data.insert(
        INTER_BOLD.to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/inter/Inter-Bold.otf"
        ))),
    );
    fonts.font_data.insert(
        LUCIDE_FONT_NAME.to_owned(),
        Arc::new(
            FontData::from_static(include_bytes!("../assets/lucide/lucide.ttf")).tweak(FontTweak {
                y_offset_factor: 0.07,
                ..Default::default()
            }),
        ),
    );
    fonts.families.insert(
        FontFamily::Proportional,
        vec![INTER_MEDIUM.to_owned(), LUCIDE_FONT_NAME.to_owned()],
    );
    fonts
        .families
        .insert(title_font_family(), vec![INTER_BOLD.to_owned()]);
    fonts
}
