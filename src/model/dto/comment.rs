//! v2 comment DTOs, separated from the page-level `Comment` entity.

use serde::{Deserialize, Serialize};

use super::super::{CommentAttachment, RichText, UserRef};
use super::ParentDto;

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
    pub display_name: Option<super::super::CommentDisplayName>,
}
