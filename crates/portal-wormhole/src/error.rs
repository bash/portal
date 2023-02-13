use magic_wormhole::transfer::TransferError;
use magic_wormhole::WormholeError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PortalError {
    #[error(transparent)]
    Wormhole(#[from] WormholeError),
    #[error(transparent)]
    WormholeTransfer(TransferError),
    #[error("Transfer rejected by peer")]
    TransferRejected(TransferError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

const TRANSFER_REJECTED_MESSAGE: &str = "transfer rejected";

impl From<TransferError> for PortalError {
    fn from(value: TransferError) -> Self {
        if let TransferError::PeerError(ref message) = value && message == TRANSFER_REJECTED_MESSAGE {
            PortalError::TransferRejected(value)
        }
        else {
            PortalError::WormholeTransfer(value)
        }
    }
}
