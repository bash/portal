use futures::stream::Aborted;
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
    #[error(transparent)]
    Walkdir(#[from] walkdir::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error("The operation has been canceled")]
    Canceled,
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

impl From<Aborted> for PortalError {
    fn from(_: Aborted) -> Self {
        PortalError::Canceled
    }
}
