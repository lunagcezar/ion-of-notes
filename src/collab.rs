//! Collaboration domain: accounts, membership, sharing, synchronization and
//! the workspace export tree.
//!
//! This is the **domain + application seam** for the self-hosted
//! client/server architecture. It contains no egui, eframe, socket or file
//! types (the domain must not know the infrastructure or the GUI). Everything
//! here is a small, serde-friendly data shape or a pure function, in keeping
//! with DRY, SOLID and the pragmatic Clean Architecture rules in `AGENTS.md`.
//!
//! The `WorkspaceApi` transport in `server.rs` exposes these shapes over the
//! decided REST + WebSocket contract (see `docs/adr-003-*`). The JSON/CSV
//! export mirrors a real workspace export: a page tree of markdown files,
//! "folder" markdown files that link to their sub-pages, and a `People` folder
//! that records workspace membership and access.

use serde::{Deserialize, Serialize};

/// Keep at most this many snapshots per page (newest first).
pub const MAX_VERSIONS_PER_PAGE: usize = 50;

mod export;

pub use export::{export_tree, page_markdown, people_csv, people_markdown};

/// Role of an account inside a workspace. Mirrors the "Membership Type" field
/// of a People entry (e.g. "Workspace owner").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Owner,
    Member,
    Guest,
}

impl Role {
    pub fn label(&self) -> &'static str {
        match self {
            Role::Owner => "Workspace owner",
            Role::Member => "Member",
            Role::Guest => "Guest",
        }
    }
}

/// Access level granted by a share. Mirrors the "Full access" / "Can edit" /
/// "Can view" / "No access" choices in the Share popover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Access {
    Full,
    Edit,
    View,
    None,
}

impl Access {
    pub fn label(&self) -> &'static str {
        match self {
            Access::Full => "Full access",
            Access::Edit => "Can edit",
            Access::View => "Can view",
            Access::None => "No access",
        }
    }

    pub fn can_write(&self) -> bool {
        matches!(self, Access::Full | Access::Edit)
    }
}

/// An account / member of the workspace (one row of the People directory).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: Role,
}

impl Member {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        email: impl Into<String>,
        role: Role,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            email: email.into(),
            role,
        }
    }
}

/// The `POST /api/members` request body (invite a member).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberDraft {
    pub name: String,
    pub email: String,
    pub role: Role,
}

/// A permission grant. `page_id == "*"` is a workspace-wide (general) rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Share {
    pub page_id: String,
    pub subject: String,
    pub access: Access,
}

/// The kind of mutation recorded in the sync log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeOp {
    Create,
    Update,
    Delete,
    Replace,
}

/// One entry of the versioned change log used to synchronize edited docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub seq: u64,
    pub page_id: String,
    pub op: ChangeOp,
    pub actor: String,
}

/// A stored version snapshot of a page (server-side version history).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageVersion {
    pub version: u64,
    pub page_id: String,
    pub title: String,
    pub actor: String,
    pub updated_at: String,
    pub snapshot: String,
}

/// A page of the sync log: all changes after a cursor plus the new cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncBatch {
    pub cursor: u64,
    pub changes: Vec<Change>,
}

/// Categories of real-time message carried over the WebSocket channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealtimeKind {
    Presence,
    Edit,
    Awareness,
    Sync,
}

/// One real-time collaboration message (the decided API payload).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeEvent {
    pub kind: RealtimeKind,
    pub page_id: String,
    pub actor: String,
    pub payload: String,
}

/// The decided real-time transport contract, exposed at `GET /api/realtime`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealtimeContract {
    pub transport: String,
    pub channel: String,
    pub events: Vec<String>,
    pub conflict_resolution: String,
}

/// Decided real-time API. Kept as data so the client and server share one
/// source of truth (DRY) instead of duplicating the protocol description.
pub fn realtime_contract() -> RealtimeContract {
    RealtimeContract {
        transport: "WebSocket over TLS, behind the self-hosted reverse proxy".to_string(),
        channel: "/ws/workspace/{workspace_id}?token=<session>".to_string(),
        events: vec![
            "presence".to_string(),
            "edit".to_string(),
            "awareness".to_string(),
            "sync".to_string(),
        ],
        conflict_resolution:
            "CRDT (Yrs/automerge-style) per block; the server persists merged snapshots and broadcasts SyncBatch cursors"
                .to_string(),
    }
}

/// One file of a workspace export (in-memory; no filesystem access here).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportFile {
    pub path: String,
    pub content: String,
}

impl ExportFile {
    fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
        }
    }
}

/// All collaboration state owned by the server: accounts, permissions, sync log,
/// and per-page version history. Persistence of this state is a separate store
/// adapter behind the `WorkspaceStore` seam; the domain itself is pure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollabState {
    pub members: Vec<Member>,
    pub shares: Vec<Share>,
    pub changes: Vec<Change>,
    pub next_seq: u64,
    #[serde(default)]
    pub versions: Vec<PageVersion>,
}

impl Default for CollabState {
    fn default() -> Self {
        Self::new()
    }
}

impl CollabState {
    /// A neutral demo workspace: one owner and two guests, with a general
    /// "everyone" share. No real identities are used (clean room).
    pub fn new() -> Self {
        Self {
            members: vec![
                Member::new(
                    "m-owner",
                    "Workspace Owner",
                    "owner@workspace.local",
                    Role::Owner,
                ),
                Member::new(
                    "m-guest-1",
                    "Alex Rivera",
                    "alex@workspace.local",
                    Role::Guest,
                ),
                Member::new("m-guest-2", "Sam Lee", "sam@workspace.local", Role::Guest),
            ],
            shares: vec![Share {
                page_id: "*".to_string(),
                subject: "Everyone at workspace".to_string(),
                access: Access::Full,
            }],
            changes: Vec::new(),
            next_seq: 1,
            versions: Vec::new(),
        }
    }

    pub fn owner_id(&self) -> String {
        self.members
            .iter()
            .find(|m| m.role == Role::Owner)
            .map(|m| m.id.clone())
            .unwrap_or_else(|| "owner".to_string())
    }

    /// Invite (register) a member and return the created account.
    pub fn invite(&mut self, draft: MemberDraft) -> Member {
        let member = Member::new(
            format!("m-{}", self.members.len() + 1),
            draft.name,
            draft.email,
            draft.role,
        );
        self.members.push(member.clone());
        member
    }

    /// Upsert a permission grant for a (page, subject) pair.
    pub fn set_access(&mut self, share: Share) {
        if let Some(slot) = self
            .shares
            .iter_mut()
            .find(|s| s.page_id == share.page_id && s.subject == share.subject)
        {
            *slot = share;
        } else {
            self.shares.push(share);
        }
    }

    /// All grants relevant to one page (page-specific plus general rules).
    pub fn shares_for(&self, page_id: &str) -> Vec<Share> {
        self.shares
            .iter()
            .filter(|s| s.page_id == page_id || s.page_id == "*")
            .cloned()
            .collect()
    }

    /// Resolve the effective access for a subject on a page (most specific
    /// first, then page-general, then workspace-general).
    pub fn access_for(&self, page_id: &str, subject: &str) -> Access {
        let specific = self
            .shares
            .iter()
            .find(|s| s.page_id == page_id && s.subject == subject);
        if let Some(s) = specific {
            return s.access;
        }
        let page_general = self.shares.iter().find(|s| {
            s.page_id == page_id && (s.subject == "*" || s.subject == "Everyone at workspace")
        });
        if let Some(s) = page_general {
            return s.access;
        }
        let workspace = self.shares.iter().find(|s| s.page_id == "*");
        if let Some(s) = workspace {
            return s.access;
        }
        Access::None
    }

    pub fn can_write(&self, page_id: &str, subject: &str) -> bool {
        self.access_for(page_id, subject).can_write()
    }

    /// Append one entry to the versioned sync log.
    pub fn record(&mut self, op: ChangeOp, page_id: &str, actor: &str) -> Change {
        let change = Change {
            seq: self.next_seq,
            page_id: page_id.to_string(),
            op,
            actor: actor.to_string(),
        };
        self.next_seq += 1;
        self.changes.push(change.clone());
        change
    }

    /// Store a page snapshot in version history.
    pub fn record_version(&mut self, page: &crate::model::Page, actor: &str) {
        let snapshot = serde_json::to_string(page).unwrap_or_default();
        let version = self
            .versions
            .iter()
            .filter(|v| v.page_id == page.id)
            .map(|v| v.version)
            .max()
            .unwrap_or(0)
            + 1;
        self.versions.push(PageVersion {
            version,
            page_id: page.id.clone(),
            title: page.title.clone(),
            actor: actor.to_string(),
            updated_at: page.updated_at.clone(),
            snapshot,
        });

        // Prune the oldest snapshots for this page only (spec-022).
        let mut excess = self
            .versions
            .iter()
            .filter(|v| v.page_id == page.id)
            .count()
            .saturating_sub(MAX_VERSIONS_PER_PAGE);
        let mut index = 0;
        while index < self.versions.len() && excess > 0 {
            if self.versions[index].page_id == page.id {
                self.versions.remove(index);
                excess -= 1;
            } else {
                index += 1;
            }
        }
    }

    pub fn versions_for(&self, page_id: &str) -> Vec<PageVersion> {
        let mut items: Vec<PageVersion> = self
            .versions
            .iter()
            .filter(|v| v.page_id == page_id)
            .cloned()
            .collect();
        items.sort_by_key(|v| std::cmp::Reverse(v.version));
        items
    }

    /// Return only changes newer than `since`, plus the new cursor.
    pub fn changes_since(&self, since: u64) -> SyncBatch {
        let changes = self
            .changes
            .iter()
            .filter(|c| c.seq > since)
            .cloned()
            .collect();
        SyncBatch {
            cursor: self.next_seq.saturating_sub(1),
            changes,
        }
    }
}
