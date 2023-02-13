#![feature(let_chains)]
#![feature(trait_alias)]

mod error;
pub mod receive;

pub use self::error::*;
mod fs;
pub mod send;
mod sync;
mod transit;

pub trait RequestRepaint = FnMut() + Clone + Send + Sync + 'static;
