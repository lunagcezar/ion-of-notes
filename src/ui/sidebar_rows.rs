//! Flattened sidebar rows for viewport-limited rendering (spec-147).
//!
//! The sidebar used to recurse over every expanded page on every frame. The
//! renderer now walks this flat list and only paints rows inside the clip rect.

use crate::model::Workspace;
use std::collections::HashSet;

/// One visible page row with its tree depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    pub page_id: String,
    pub depth: usize,
    pub has_children: bool,
    pub open: bool,
}

/// Flatten the expanded, non-archived page tree in display order.
pub fn visible_tree_rows(workspace: &Workspace, expanded: &HashSet<String>) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    collect(workspace, None, 0, expanded, &mut rows);
    rows
}

fn collect(
    workspace: &Workspace,
    parent: Option<&str>,
    depth: usize,
    expanded: &HashSet<String>,
    rows: &mut Vec<TreeRow>,
) {
    for page in workspace.children(parent) {
        if page.archived {
            continue;
        }
        let has_children = !workspace.children(Some(&page.id)).is_empty();
        let open = expanded.contains(&page.id);
        rows.push(TreeRow {
            page_id: page.id.clone(),
            depth,
            has_children,
            open,
        });
        if open {
            collect(workspace, Some(&page.id), depth + 1, expanded, rows);
        }
    }
}
