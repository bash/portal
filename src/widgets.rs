mod cancel;
pub use self::cancel::*;
mod page;
pub use self::page::*;
mod tab_button;
pub use self::tab_button::*;
mod primary_button;
mod view_switcher;
pub use self::primary_button::*;
use egui::Vec2;
pub use view_switcher::*;

pub const MIN_BUTTON_SIZE: Vec2 = Vec2::new(100.0, 0.0);
