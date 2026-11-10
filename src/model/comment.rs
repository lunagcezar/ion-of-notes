//! Page comments and comment DTOs.

use serde::{Deserialize, Serialize};

use super::block::ParentDto;
use super::file::CommentAttachment;
use super::rich_text::RichText;
use super::user::UserRef;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommentDisplayName {
    Connection,
    User,
    Custom { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentDisplayNameDto {
    pub r#type: String,
    pub resolved_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentDto {
    pub object: String,
    pub id: String,
    pub parent: ParentDto,
    pub discussion_id: String,
    #[serde(default)]
    pub property_id: Option<String>,
    pub created_time: String,
    pub last_edited_time: String,
    pub created_by: UserRef,
    #[serde(default)]
    pub rich_text: Vec<RichText>,
    #[serde(default)]
    pub attachments: Vec<CommentAttachment>,
    #[serde(default)]
    pub display_name: Option<CommentDisplayNameDto>,
    #[serde(default)]
    pub has_pending_attachments: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentInput {
    pub file_upload_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentInput {
    pub parent: ParentDto,
    #[serde(default)]
    pub discussion_id: Option<String>,
    #[serde(default)]
    pub property_id: Option<String>,
    #[serde(default)]
    pub rich_text: Vec<RichText>,
    #[serde(default)]
    pub attachments: Vec<AttachmentInput>,
    #[serde(default)]
    pub display_name: Option<CommentDisplayName>,
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
