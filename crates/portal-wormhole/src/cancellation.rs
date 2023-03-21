//! A generalized solution for cancellation.
//! This is not optimized at all, but it's very convenient to use.
//!
//! The main issue this tries to solve is provide a unified API for
//! different forms of cancellation used in this project:
//!
//! * Cancellation using futures' [`AbortHandle`]
//! * Cancellation in synchronous code.
//! * Cancellation using a future (completion means cancellation).
//!
//! The API and implementation is inspired by .NET's `CancellationToken`:
//! There's a cancellation token which is passed to cancelable functions and a cancellation source that controls cancellation.
//! Consumers can register themselves with the cancellation token for cancellation.

use futures::channel::oneshot;
use futures::future::{AbortHandle, AbortRegistration};
use static_assertions::assert_impl_all;
use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::{fmt, mem};
use thiserror::Error;

#[derive(Debug, Clone)]
pub(crate) struct CancellationToken {
    inner: Arc<CancellationInner>,
}

impl CancellationToken {
    pub(crate) fn is_canceled(&self) -> bool {
        self.inner.canceled.load(Ordering::Relaxed)
    }

    pub(crate) fn error_if_canceled(&self) -> Result<(), CancellationError> {
        if self.is_canceled() {
            Err(CancellationError)
        } else {
            Ok(())
        }
    }

    /// Registers a [`FnOnce`] to be called on cancellation.
    /// The func is called immediately if this token is already canceled.
    pub(crate) fn register(&self, func: impl FnOnce() + Send + Sync + 'static) {
        let mut funcs = self.inner.funcs.write().unwrap();

        if self.is_canceled() {
            func();
        } else {
            funcs.push(Box::new(func));
        }
    }
}

impl CancellationToken {
    pub(crate) fn as_abort_registration(&self) -> AbortRegistration {
        let (handle, registration) = AbortHandle::new_pair();
        self.register(move || handle.abort());
        registration
    }

    pub(crate) fn as_future(&self) -> impl Future<Output = ()> {
        let (tx, rx) = oneshot::channel::<()>();
        self.register(move || {
            _ = tx.send(());
        });
        async { _ = rx.await }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CancellationSource {
    inner: Arc<CancellationInner>,
}

impl CancellationSource {
    pub(crate) fn token(&self) -> CancellationToken {
        CancellationToken {
            inner: self.inner.clone(),
        }
    }

    pub(crate) fn cancel(&self) {
        let mut funcs = self.inner.funcs.write().unwrap();
        self.inner.canceled.store(true, Ordering::Relaxed);
        let funcs = mem::take(&mut *funcs);
        for func in funcs {
            func();
        }
    }
}

#[derive(Default)]
struct CancellationInner {
    canceled: AtomicBool,
    funcs: RwLock<Vec<Box<dyn FnOnce() + Send + Sync>>>, // TODO: This + Sync is needed for RwLock to be Sync, but can we work around that somehow?
}

impl fmt::Debug for CancellationInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CancellationInner")
            .field("canceled", &self.canceled.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

#[derive(Error, Debug, Default)]
#[error("")]
pub(crate) struct CancellationError;

assert_impl_all!(CancellationToken: Send);
assert_impl_all!(CancellationSource: Send);
