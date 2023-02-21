#![feature(let_chains)]
#![feature(trait_alias)]
#![feature(path_as_mut_os_str)]

mod error;
pub mod receive;

pub use self::error::*;
mod fs;
pub mod send;
mod sync;
mod temp_zip;
mod transit;

pub use magic_wormhole::transit::TransitInfo;
pub use magic_wormhole::Code;

pub trait RequestRepaint = FnMut() + Clone + Send + Sync + 'static;

#[derive(Default, Copy, Clone)]
pub struct Progress {
    pub value: u64,
    pub total: u64,
}
