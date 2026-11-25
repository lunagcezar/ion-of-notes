//! Mutex helpers that log and count poisoning instead of silently ignoring it
//! (spec-155).
//!
//! Recovering is still safer than panicking in this single-process server, but
//! a poisoned mutex means another thread panicked while holding it; the event
//! must be observable through logs and `/metrics`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

/// Number of times a mutex was found poisoned and recovered.
pub(crate) static POISONED_LOCKS: AtomicU64 = AtomicU64::new(0);

/// Lock a mutex, logging and counting a poisoned state before recovering.
pub(crate) trait LockExt<T> {
    fn lock_or_log(&self) -> MutexGuard<'_, T>;
}

impl<T> LockExt<T> for Mutex<T> {
    fn lock_or_log(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                POISONED_LOCKS.fetch_add(1, Ordering::Relaxed);
                tracing::error!("mutex poisoned; recovering the inner state");
                poisoned.into_inner()
            }
        }
    }
}
