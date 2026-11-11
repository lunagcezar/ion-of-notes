//! A persisted catalog of workspaces for the desktop client (spec-058).
//!
//! The server still owns one active workspace at a time. The catalog keeps the
//! other workspace snapshots beside it, so the UI can create, switch, rename
//! and delete workspaces without a multi-tenant server.

use serde::{Deserialize, Serialize};

use super::Workspace;

/// A list row for the workspace switcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub active: bool,
    pub page_count: usize,
}

/// All known workspaces and the active one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceCatalog {
    #[serde(default)]
    pub active_id: String,
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
}

impl WorkspaceCatalog {
    pub fn new(workspace: Workspace) -> Self {
        let active_id = workspace.settings.id.clone();
        Self {
            active_id,
            workspaces: vec![workspace],
        }
    }

    pub fn active(&self) -> Option<&Workspace> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.settings.id == self.active_id)
            .or_else(|| self.workspaces.first())
    }

    pub fn summaries(&self) -> Vec<WorkspaceSummary> {
        self.workspaces
            .iter()
            .map(|workspace| WorkspaceSummary {
                id: workspace.settings.id.clone(),
                name: workspace.name.clone(),
                icon: workspace.settings.icon.clone(),
                active: workspace.settings.id == self.active_id,
                page_count: workspace.pages.len(),
            })
            .collect()
    }

    /// Replace one workspace snapshot by its id, or add it when new.
    pub fn replace(&mut self, workspace: Workspace) {
        let id = workspace.settings.id.clone();
        if let Some(slot) = self
            .workspaces
            .iter_mut()
            .find(|item| item.settings.id == id)
        {
            *slot = workspace;
        } else {
            self.workspaces.push(workspace);
        }
    }

    pub fn add(&mut self, workspace: Workspace) -> bool {
        if self
            .workspaces
            .iter()
            .any(|item| item.settings.id == workspace.settings.id)
        {
            return false;
        }
        self.active_id = workspace.settings.id.clone();
        self.workspaces.push(workspace);
        true
    }

    pub fn switch(&mut self, id: &str) -> bool {
        if self.workspaces.iter().any(|item| item.settings.id == id) {
            self.active_id = id.to_string();
            true
        } else {
            false
        }
    }

    /// Remove a workspace. The last workspace cannot be removed.
    pub fn remove(&mut self, id: &str) -> bool {
        if self.workspaces.len() <= 1 {
            return false;
        }
        let Some(index) = self
            .workspaces
            .iter()
            .position(|item| item.settings.id == id)
        else {
            return false;
        };
        self.workspaces.remove(index);
        if self.active_id == id
            && let Some(first) = self.workspaces.first()
        {
            self.active_id = first.settings.id.clone();
        }
        true
    }
}
