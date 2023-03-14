use portal_wormhole::{Code, WormholeTransferUri};
use std::error::Error;
use std::str::FromStr;
use thiserror::Error;

#[derive(Default, Debug)]
pub enum StartupAction {
    #[default]
    None,
    ReceiveFile(ReceiveFileAction),
    ShowInvalidUriError(Box<dyn Error>),
}

#[derive(Debug)]
pub struct ReceiveFileAction {
    pub code: Code,
}

impl StartupAction {
    pub fn from_uri(uri: Option<&str>) -> Self {
        uri.map(Self::from_uri_str).unwrap_or_default()
    }

    fn from_uri_str(uri: &str) -> Self {
        WormholeTransferUri::from_str(uri)
            .map(Self::from_wormhole_transfer_uri)
            .unwrap_or_else(|error| StartupAction::ShowInvalidUriError(error.into()))
    }

    fn from_wormhole_transfer_uri(uri: WormholeTransferUri) -> Self {
        if uri.is_leader || uri.rendezvous_server.is_some() {
            StartupAction::ShowInvalidUriError(UnsupportedWormholeUriError(uri).into())
        } else {
            StartupAction::ReceiveFile(ReceiveFileAction { code: uri.code })
        }
    }
}

#[derive(Error, Debug)]
#[error("Unsupported wormhole-transfer URI: {}", ToString::to_string(.0))]
struct UnsupportedWormholeUriError(WormholeTransferUri);
