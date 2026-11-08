//! Derived lookup index for a workspace.
//!
//! The index maps page ids to their position and parents to their child ids.
//! It is rebuilt when pages are added or removed and after deserialization;
//! `Workspace` falls back to a linear scan if a direct `pages` mutation made
//! the index stale (spec-022).

use std::collections::HashMap;

use super::Page;

#[derive(Debug, Clone, Default)]
pub(super) struct WorkspaceIndex {
    positions: HashMap<String, usize>,
    children: HashMap<String, Vec<String>>,
    roots: Vec<String>,
}

impl WorkspaceIndex {
    pub(super) fn build(pages: &[Page]) -> Self {
        let mut index = Self::default();
        for (position, page) in pages.iter().enumerate() {
            index.positions.insert(page.id.clone(), position);
            match &page.parent {
                Some(parent) => index
                    .children
                    .entry(parent.clone())
                    .or_default()
                    .push(page.id.clone()),
                None => index.roots.push(page.id.clone()),
            }
        }
        index
    }

    pub(super) fn len(&self) -> usize {
        self.positions.len()
    }

    pub(super) fn position(&self, id: &str) -> Option<usize> {
        self.positions.get(id).copied()
    }

    pub(super) fn children(&self, parent: Option<&str>) -> &[String] {
        match parent {
            Some(id) => self.children.get(id).map(Vec::as_slice).unwrap_or(&[]),
            None => &self.roots,
        }
    }
}
