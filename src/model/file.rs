//! File objects, uploads and comment attachments.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileObject {
    File {
        url: String,
        #[serde(default)]
        expiry_time: Option<String>,
    },
    FileUpload {
        id: String,
    },
    External {
        url: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum UploadStatus {
    #[default]
    Pending,
    Uploaded,
    Expired,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileUpload {
    pub object: String,
    pub id: String,
    pub created_time: String,
    pub last_edited_time: String,
    #[serde(default)]
    pub expiry_time: Option<String>,
    pub status: UploadStatus,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub content_length: Option<u64>,
    #[serde(default)]
    pub upload_url: Option<String>,
    #[serde(default)]
    pub complete_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FileUploadInput {
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub content_length: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentAttachment {
    pub category: String,
    pub file: FileObject,
}
