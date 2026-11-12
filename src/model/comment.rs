//! Page-level comments and the comment display-name rule.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommentDisplayName {
    Connection,
    User,
    Custom { name: String },
}

/// One page-level discussion comment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub author: String,
    pub text: String,
    pub created_at: String,
}

impl Comment {
    pub fn new(
        id: impl Into<String>,
        author: impl Into<String>,
        text: impl Into<String>,
        created_at: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            author: author.into(),
            text: text.into(),
            created_at: created_at.into(),
        }
    }
}
