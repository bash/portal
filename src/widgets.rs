mod cancel;
pub use self::cancel::*;
mod page;
pub use self::page::*;
mod primary_button;
pub use self::primary_button::*;
use egui::Vec2;
mod toggle;
pub use toggle::*;

pub const MIN_BUTTON_SIZE: Vec2 = Vec2::new(100.0, 0.0);
