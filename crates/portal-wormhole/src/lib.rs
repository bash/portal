mod error;
pub mod receive;
pub use self::error::*;
mod cancellation;
mod fs;
pub mod send;
mod sync;
mod temp_zip;
mod transit;

pub use magic_wormhole::transit::{ConnectionType, TransitInfo};
pub use magic_wormhole::uri::WormholeTransferUri;
pub use magic_wormhole::Code;
use std::fmt;
use trait_set::trait_set;
use url::Url;

trait_set! {
    pub trait RequestRepaint = FnMut() + Clone + Send + Sync + 'static;
}

#[derive(Default, Copy, Clone)]
pub struct Progress {
    pub value: u64,
    pub total: u64,
}

#[non_exhaustive]
pub struct SharableWormholeTransferUri {
    pub code: Code,
}

impl SharableWormholeTransferUri {
    pub fn new(code: Code) -> Self {
        Self { code }
    }
}

impl From<&SharableWormholeTransferUri> for Url {
    fn from(value: &SharableWormholeTransferUri) -> Self {
        let mut url =
            Url::parse("https://wormhole-transfer.link").expect("constant URL should be valid");
        url.set_fragment(Some(&value.code));
        url
    }
}

impl fmt::Display for SharableWormholeTransferUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let uri: Url = self.into();
        write!(f, "{}", uri)
    }
}
