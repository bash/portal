use magic_wormhole::WormholeError;
use thiserror::Error;

#[derive(Error, Debug)]
#[error(transparent)]
pub enum PortalError {
    Wormhole(#[from] WormholeError),
    WormholeTransfer(#[from] magic_wormhole::transfer::TransferError),
    Io(#[from] std::io::Error),
}
