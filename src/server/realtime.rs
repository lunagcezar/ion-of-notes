//! Real-time change feed and presence hub (spec-040).
//!
//! This module is transport-neutral. The Axum WebSocket adapter consumes the
//! typed messages; tests can exercise the hub without a socket.

use crate::collab::Change;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

/// Server-to-client WebSocket message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Ready {
        workspace_id: String,
        session_id: String,
        seq: u64,
    },
    Change {
        change: Change,
    },
    Presence {
        members: Vec<PresenceMember>,
    },
    Pong,
    Error {
        code: String,
        message: String,
    },
}

/// Client-to-server WebSocket message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping,
}

/// One active real-time participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresenceMember {
    pub session_id: String,
    pub member_id: String,
    pub name: String,
    pub state: String,
}

/// Broadcast hub shared by the server and every WebSocket session.
#[derive(Clone)]
pub struct RealtimeHub {
    sender: broadcast::Sender<ServerMessage>,
    presence: Arc<Mutex<BTreeMap<String, PresenceMember>>>,
}

impl RealtimeHub {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            presence: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.sender.subscribe()
    }

    /// Publish one committed change to every subscriber.
    pub fn publish_change(&self, change: Change) {
        let _ = self.sender.send(ServerMessage::Change { change });
    }

    /// Add a participant and broadcast the new presence list.
    pub fn join(&self, member: PresenceMember) {
        self.presence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(member.session_id.clone(), member);
        self.publish_presence();
    }

    /// Remove a participant and broadcast the new presence list.
    pub fn leave(&self, session_id: &str) {
        self.presence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(session_id);
        self.publish_presence();
    }

    /// Current participants, ordered by session id.
    pub fn members(&self) -> Vec<PresenceMember> {
        self.presence
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .cloned()
            .collect()
    }

    fn publish_presence(&self) {
        let members = self.members();
        let _ = self.sender.send(ServerMessage::Presence { members });
    }
}
