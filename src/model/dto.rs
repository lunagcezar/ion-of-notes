//! API-facing data shapes, separated from the domain entities.
//!
//! The domain (`Block`, `Comment`, `Page`, ...) describes workspace rules and
//! storage. The DTOs here describe the v2 wire contract and are free to evolve
//! with it. Conversion lives next to the DTO so the domain files stay free of
//! serialization concerns beyond their own persistence shape (spec-099).

mod block;
mod comment;

pub use block::{BlockChildrenRequest, BlockDto, BlockInput, ParentDto};
pub use comment::{AttachmentInput, CommentDisplayNameDto, CommentDto, CommentInput};
