//! Members, shares, sync, export and realtime-contract use cases.

use crate::collab::{
    ExportFile, Member, MemberDraft, RealtimeContract, Share, SyncBatch, export_tree,
    realtime_contract as realtime_contract_value,
};

use super::super::WorkspaceServer;
use super::UseCaseError;

/// The people directory.
pub(in crate::server) fn list_members(server: &WorkspaceServer) -> Vec<Member> {
    server.collab.members.clone()
}

/// Invite one member and persist the collaboration document.
pub(in crate::server) fn invite_member(
    server: &mut WorkspaceServer,
    draft: MemberDraft,
) -> Result<Member, UseCaseError> {
    if draft.name.trim().is_empty() || draft.email.trim().is_empty() {
        return Err(UseCaseError::Invalid(
            "member name and email must not be empty".to_string(),
        ));
    }
    let member = server.collab.invite(draft);
    server
        .persist_collab()
        .map_err(|error| UseCaseError::Storage(error.to_string()))?;
    Ok(member)
}

/// Permission grants, optionally filtered by page.
pub(in crate::server) fn list_shares(server: &WorkspaceServer, page: Option<&str>) -> Vec<Share> {
    match page {
        Some(page) if !page.is_empty() => server.collab.shares_for(page),
        _ => server.collab.shares.clone(),
    }
}

/// Set one permission grant and persist it.
pub(in crate::server) fn set_share(
    server: &mut WorkspaceServer,
    share: Share,
) -> Result<Share, UseCaseError> {
    server.collab.set_access(share.clone());
    server
        .persist_collab()
        .map_err(|error| UseCaseError::Storage(error.to_string()))?;
    Ok(share)
}

/// Changes after a cursor.
pub(in crate::server) fn sync(server: &WorkspaceServer, since: u64) -> SyncBatch {
    server.collab.changes_since(since)
}

/// The Notion-style export tree.
pub(in crate::server) fn export(server: &WorkspaceServer) -> Vec<ExportFile> {
    export_tree(&server.workspace, &server.collab.members)
}

/// The decided realtime/CRDT contract as data.
pub(in crate::server) fn realtime_contract() -> RealtimeContract {
    realtime_contract_value()
}
