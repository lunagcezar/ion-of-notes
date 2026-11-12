//! Service modules that extend `WorkspaceServer` with one cohesive concern.
//!
//! The methods are still inherent to the server for now; the module boundary
//! keeps auth and webhook logic out of the generic handler tree and is the
//! seam for a later owned-service extraction.

mod auth;
mod webhooks;
