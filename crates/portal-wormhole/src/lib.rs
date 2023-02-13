#![feature(let_chains)]
#![feature(trait_alias)]

mod error;
pub mod receive;
pub use self::error::*;
mod defaults;
mod fs;
pub mod send;
mod sync;

pub trait RequestRepaint = FnMut() + Clone + Send + 'static;
