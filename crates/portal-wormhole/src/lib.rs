#![feature(let_chains)]
#![feature(trait_alias)]

mod error;
pub mod receive;

pub use self::error::*;
mod fs;
pub mod send;
mod sync;
mod transit;

pub use magic_wormhole::{Code, transit::TransitInfo};

pub trait RequestRepaint = FnMut() + Clone + Send + Sync + 'static;

#[derive(Default, Copy, Clone)]
pub struct Progress {
    pub value: u64,
    pub total: u64,
}
