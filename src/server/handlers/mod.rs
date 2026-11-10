//! One module per v2 resource family. Each module adds inherent methods to
//! `WorkspaceServer`; the route adapters in `server::routes` call them.

mod auth;
mod blocks;
mod content;
mod crdt;
mod databases;
mod lists;
mod views;
mod webhooks;
