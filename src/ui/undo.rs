//! Undo/redo entries that avoid cloning the whole workspace for typing
//! (spec-146).
//!
//! A frame that only edits the current page keeps a patch with the page before
//! and after the edit. Clicks, modal overlays and keyboard commands still take a
//! full snapshot because they can change page order, the tree or other pages.

use crate::model::{Page, Workspace};

/// What was captured before a frame rendered.
#[derive(Debug, Clone)]
pub enum UndoCapture {
    Snapshot(Workspace),
    Page { page_id: String, page: Option<Page> },
}

/// One reversible operation.
#[derive(Debug, Clone)]
pub enum UndoEntry {
    Snapshot(Workspace),
    Page {
        page_id: String,
        before: Option<Page>,
        after: Option<Page>,
    },
}

impl UndoEntry {
    /// Build the entry for a frame, or `None` when nothing changed.
    pub fn from_capture(capture: UndoCapture, workspace: &Workspace) -> Option<Self> {
        match capture {
            UndoCapture::Snapshot(before) => {
                (before != *workspace).then_some(UndoEntry::Snapshot(before))
            }
            UndoCapture::Page { page_id, page } => {
                let after = workspace.page(&page_id).cloned();
                (after != page).then_some(UndoEntry::Page {
                    page_id,
                    before: page,
                    after,
                })
            }
        }
    }

    /// Apply the "before" side (undo).
    pub fn undo(&self, workspace: &mut Workspace) {
        match self {
            UndoEntry::Snapshot(before) => *workspace = before.clone(),
            UndoEntry::Page {
                page_id, before, ..
            } => restore(workspace, page_id, before),
        }
        workspace.blocks_changed();
    }

    /// Apply the "after" side (redo).
    pub fn redo(&self, workspace: &mut Workspace) {
        match self {
            UndoEntry::Snapshot(after) => *workspace = after.clone(),
            UndoEntry::Page { page_id, after, .. } => restore(workspace, page_id, after),
        }
        workspace.blocks_changed();
    }
}

fn restore(workspace: &mut Workspace, page_id: &str, page: &Option<Page>) {
    // Page patches are only recorded for edits that keep the page in place;
    // structural commands (delete/add/reorder) use a full snapshot.
    if let Some(page) = page
        && let Some(slot) = workspace.page_mut(page_id)
    {
        *slot = page.clone();
    }
}
