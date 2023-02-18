use oneshot::TryRecvError;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use BorrowingOneshotReceiverState::*;

use crate::PortalError;

pub struct BorrowingOneshotReceiver<T> {
    state: BorrowingOneshotReceiverState<T>,
    incoming: oneshot::Receiver<T>,
}

enum BorrowingOneshotReceiverState<T> {
    Waiting,
    Completed(T),
    Disconnected,
}

impl<T> BorrowingOneshotReceiverState<T> {
    fn value(&self) -> Option<&T> {
        match self {
            Completed(ref value) => Some(value),
            _ => None,
        }
    }
}

impl<T> BorrowingOneshotReceiver<T> {
    pub fn value(&mut self) -> Option<&T> {
        self.try_recv();
        self.state.value()
    }

    fn try_recv(&mut self) {
        if matches!(self.state, Waiting) {
            self.state = match self.incoming.try_recv() {
                Ok(value) => Completed(value),
                Err(TryRecvError::Empty) => Waiting,
                Err(TryRecvError::Disconnected) => Disconnected,
            }
        }
    }
}

impl<T> From<oneshot::Receiver<T>> for BorrowingOneshotReceiver<T> {
    fn from(value: oneshot::Receiver<T>) -> Self {
        BorrowingOneshotReceiver {
            state: BorrowingOneshotReceiverState::Waiting,
            incoming: value,
        }
    }
}

pub fn cancellation_pair() -> (CancellationSender, CancellationReceiver) {
    let canceled = Arc::new(AtomicBool::new(false));
    (
        CancellationSender {
            canceled: canceled.clone(),
        },
        CancellationReceiver { canceled },
    )
}

pub struct CancellationReceiver {
    canceled: Arc<AtomicBool>,
}

impl CancellationReceiver {
    pub fn propagate(&self) -> Result<(), PortalError> {
        if self.canceled.load(Ordering::Relaxed) {
            Err(PortalError::Canceled)
        } else {
            Ok(())
        }
    }
}

pub struct CancellationSender {
    canceled: Arc<AtomicBool>,
}

impl CancellationSender {
    pub fn cancel(&self) {
        self.canceled.store(true, Ordering::Relaxed);
    }
}
