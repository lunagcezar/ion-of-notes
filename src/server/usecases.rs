//! Application use cases shared by the v1 and v2 route adapters.
//!
//! A use case validates, mutates and persists through `WorkspaceServer`, then
//! returns a typed result or a `UseCaseError`. It never builds an envelope or
//! chooses an HTTP status: the adapters in `server::routes` own that mapping.

mod comments;
mod error;
mod pages;
mod sharing;
mod workspace;

pub(super) use comments::{create_comment, delete_comment, list_comments, update_comment};
pub(super) use error::UseCaseError;
pub(super) use pages::{create_page, delete_page, get_page, page_versions, update_page};
pub(super) use sharing::{
    export, invite_member, issue_invite, list_members, list_shares, realtime_contract, set_share,
};
pub(super) use workspace::replace_workspace;
