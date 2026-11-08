//! Pure persistence-write planning and debounce (spec-020).
//!
//! The UI keeps the last successfully committed workspace. This module decides
//! whether a diff is empty, needs one or more page `PATCH` requests, or needs
//! the full-workspace fallback, and when a quiet period has elapsed.

use crate::client::Client;
use crate::model::{PageDiff, Workspace};
use crate::server::WorkspaceApi;

/// The request shape for one commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitPlan {
    None,
    Update(Vec<String>),
    Replace,
}

/// Decide how to persist a workspace diff.
pub fn plan_for(diff: &PageDiff) -> CommitPlan {
    if diff.is_empty() {
        CommitPlan::None
    } else if diff.requires_full_replace() {
        CommitPlan::Replace
    } else {
        CommitPlan::Update(diff.changed.clone())
    }
}

/// Send one commit plan through the client. Returns false when any request
/// fails, so the caller can retry after the next debounce window.
pub fn execute_plan<A: WorkspaceApi>(
    client: &mut Client<A>,
    workspace: &Workspace,
    plan: &CommitPlan,
) -> bool {
    match plan {
        CommitPlan::None => true,
        CommitPlan::Update(ids) => ids.iter().all(|id| {
            workspace
                .page(id)
                .map(|page| client.update_page(page).is_ok())
                .unwrap_or(false)
        }),
        CommitPlan::Replace => client.replace_workspace(workspace).is_ok(),
    }
}

/// Debounce state for pending edits.
#[derive(Debug, Clone)]
pub struct PendingEdits {
    last_change: Option<f64>,
    debounce: f64,
    force: bool,
    last_diff: PageDiff,
}

impl PendingEdits {
    /// Create a debounce window in seconds.
    pub fn new(debounce_secs: f64) -> Self {
        Self {
            last_change: None,
            debounce: debounce_secs,
            force: false,
            last_diff: PageDiff::default(),
        }
    }

    /// Record a change at `now` (seconds). Repeated marks extend the window.
    pub fn mark(&mut self, now: f64) {
        self.last_change = Some(now);
    }

    /// Observe a workspace diff. The debounce timer is reset only when the
    /// diff actually changes, so an unchanged frame cannot keep postponing the
    /// flush forever.
    pub fn observe(&mut self, diff: &PageDiff, now: f64) {
        if &self.last_diff != diff {
            self.last_diff = diff.clone();
            self.last_change = Some(now);
        }
    }

    /// Request an immediate flush on the next check.
    pub fn force(&mut self) {
        self.force = true;
    }

    /// True when the debounce elapsed or a flush was forced.
    pub fn should_flush(&self, now: f64) -> bool {
        self.force
            || self
                .last_change
                .is_some_and(|changed_at| now - changed_at >= self.debounce)
    }

    /// True when there is an unflushed change.
    pub fn is_pending(&self) -> bool {
        self.last_change.is_some() || self.force
    }

    /// Clear the pending state after a successful flush.
    pub fn flushed(&mut self) {
        self.last_change = None;
        self.force = false;
        self.last_diff = PageDiff::default();
    }
}
