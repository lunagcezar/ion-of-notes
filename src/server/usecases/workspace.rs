//! Whole-workspace read and replace use cases.

use crate::collab::ChangeOp;
use crate::model::Workspace;

use super::super::WorkspaceServer;
use super::UseCaseError;

/// The current workspace snapshot.
pub(in crate::server) fn get_workspace(server: &WorkspaceServer) -> Workspace {
    server.workspace.clone()
}

/// Replace the workspace, record changed pages, persist and return the result.
pub(in crate::server) fn replace_workspace(
    server: &mut WorkspaceServer,
    workspace: Workspace,
) -> Result<Workspace, UseCaseError> {
    let actor = server.collab.owner_id();
    for page in &workspace.pages {
        let changed = server
            .workspace
            .page(&page.id)
            .map(|old| old != page)
            .unwrap_or(true);
        if changed {
            server.collab.record_version(page, &actor);
        }
    }
    server.workspace = workspace;
    server
        .write(ChangeOp::Replace, "__workspace__")
        .map_err(|error| UseCaseError::Storage(error.to_string()))?;
    Ok(server.workspace.clone())
}
