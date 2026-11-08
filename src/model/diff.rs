//! Workspace diffing for the persistence write path.
//!
//! The UI compares the last committed snapshot with the current one to decide
//! whether a single-page `PATCH` is enough or a full-workspace `PUT` is
//! required (spec-020).

use std::collections::HashMap;

use super::{Page, Workspace};

/// The difference between two workspace snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PageDiff {
    /// Pages whose content changed, in the `after` page order.
    pub changed: Vec<String>,
    /// Pages present only in `after`, in the `after` page order.
    pub added: Vec<String>,
    /// Pages present only in `before`, in the `before` page order.
    pub removed: Vec<String>,
    /// True when the same set of pages appears in a different order.
    pub reordered: bool,
    /// True when the workspace name or workspace settings changed.
    pub settings_changed: bool,
}

impl PageDiff {
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty()
            && self.added.is_empty()
            && self.removed.is_empty()
            && !self.reordered
            && !self.settings_changed
    }

    /// A structural or workspace-level change needs the full snapshot.
    pub fn requires_full_replace(&self) -> bool {
        !self.added.is_empty()
            || !self.removed.is_empty()
            || self.reordered
            || self.settings_changed
    }
}

impl Workspace {
    /// Compare `self` (the last committed state) with `after` (the current UI
    /// state) without cloning either workspace.
    pub fn diff(&self, after: &Workspace) -> PageDiff {
        let before_by_id: HashMap<&str, &Page> =
            self.pages.iter().map(|p| (p.id.as_str(), p)).collect();
        let after_by_id: HashMap<&str, &Page> =
            after.pages.iter().map(|p| (p.id.as_str(), p)).collect();

        let mut diff = PageDiff::default();
        for page in &after.pages {
            match before_by_id.get(page.id.as_str()) {
                None => diff.added.push(page.id.clone()),
                Some(old) if *old != page => diff.changed.push(page.id.clone()),
                _ => {}
            }
        }
        for page in &self.pages {
            if !after_by_id.contains_key(page.id.as_str()) {
                diff.removed.push(page.id.clone());
            }
        }

        let same_ids = before_by_id.len() == after_by_id.len()
            && after_by_id.keys().all(|id| before_by_id.contains_key(id));
        diff.reordered = same_ids
            && !self
                .pages
                .iter()
                .map(|p| p.id.as_str())
                .eq(after.pages.iter().map(|p| p.id.as_str()));
        diff.settings_changed = self.name != after.name || self.settings != after.settings;
        diff
    }
}
