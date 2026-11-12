//! One module per v2 resource family. Each module adds inherent methods to
//! `WorkspaceServer`; the route adapters in `server::routes` call them.

mod blocks;
mod comments;
mod crdt;
mod databases;
mod lists;
mod shared;
mod user_file;
mod views;
