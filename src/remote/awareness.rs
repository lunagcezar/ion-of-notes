//! Latest cursor/selection that could not be sent over the WebSocket.
//!
//! The slot is intentionally bounded to one entry: while the socket is down,
//! only the most recent cursor matters. The reconnect loop flushes it when a
//! connection is established.

use crate::server::realtime::Awareness;
use std::sync::Mutex;

/// A one-entry mailbox for the latest awareness update.
#[derive(Debug, Default)]
pub struct PendingAwareness {
    slot: Mutex<Option<Awareness>>,
}

impl PendingAwareness {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the previous cursor with this one.
    pub fn record(&self, awareness: Awareness) {
        let mut slot = self
            .slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = Some(awareness);
    }

    /// The cursor waiting to be sent, if any.
    pub fn snapshot(&self) -> Option<Awareness> {
        self.slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Clear the slot only if it still holds `expected`, so a cursor recorded
    /// while a send was in flight is not lost.
    pub fn clear_if(&self, expected: &Awareness) {
        let mut slot = self
            .slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot.as_ref() == Some(expected) {
            *slot = None;
        }
    }
}
