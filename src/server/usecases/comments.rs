//! Comment use cases shared by the v1 and v2 adapters.

use super::super::WorkspaceServer;
use super::super::events::DomainEvent;
use super::UseCaseError;
use crate::collab::ChangeOp;
use crate::model::{
    AttachmentInput, CommentAttachment, CommentDisplayName, CommentDisplayNameDto, CommentDto,
    CommentInput, FileObject, ParentDto, UploadStatus, now_iso,
};

/// The documented per-comment attachment limit.
pub(in crate::server) const MAX_COMMENT_ATTACHMENTS: usize = 3;

pub(in crate::server) fn list_comments(
    server: &WorkspaceServer,
    page_filter: &str,
    block_filter: &str,
) -> Vec<CommentDto> {
    let mut comments: Vec<CommentDto> = server
        .workspace
        .comments
        .iter()
        .filter(|comment| {
            if !page_filter.is_empty() {
                return matches!(
                    &comment.parent,
                    ParentDto::Page { page_id } if page_id == page_filter
                );
            }
            if !block_filter.is_empty() {
                return matches!(
                    &comment.parent,
                    ParentDto::Block { block_id } if block_id == block_filter
                );
            }
            true
        })
        .cloned()
        .collect();
    comments.sort_by(|a, b| a.created_time.cmp(&b.created_time));
    comments
}

pub(in crate::server) fn create_comment(
    server: &mut WorkspaceServer,
    input: CommentInput,
) -> Result<CommentDto, UseCaseError> {
    if input.attachments.len() > MAX_COMMENT_ATTACHMENTS {
        return Err(UseCaseError::Invalid(format!(
            "a comment supports at most {MAX_COMMENT_ATTACHMENTS} attachments"
        )));
    }
    let page_id = resolve_parent_page(server, &input.parent)?;
    let attachments = resolve_attachments(server, &input.attachments)?;
    if input.rich_text.is_empty() && attachments.is_empty() {
        return Err(UseCaseError::Invalid(
            "a comment needs text or an attachment".to_string(),
        ));
    }
    let now = now_iso();
    let comment = CommentDto {
        object: "comment".to_string(),
        id: format!("comment-{}", uuid::Uuid::new_v4()),
        parent: input.parent,
        discussion_id: input
            .discussion_id
            .unwrap_or_else(|| format!("discussion-{}", uuid::Uuid::new_v4())),
        property_id: input.property_id,
        created_time: now.clone(),
        last_edited_time: now,
        created_by: crate::model::UserRef::new(server.collab.owner_id()),
        rich_text: input.rich_text,
        attachments,
        display_name: input
            .display_name
            .as_ref()
            .map(|display| resolve_display_name(server, display)),
        has_pending_attachments: false,
    };
    server.workspace.comments.push(comment.clone());
    server
        .write(ChangeOp::Create, &page_id)
        .map_err(|error| UseCaseError::Storage(error.to_string()))?;
    server.notify(DomainEvent::CommentCreated {
        comment_id: comment.id.clone(),
        data: serde_json::json!({ "page_id": page_id, "parent": comment.parent }).to_string(),
    });
    Ok(comment)
}

pub(in crate::server) fn update_comment(
    server: &mut WorkspaceServer,
    id: &str,
    input: CommentInput,
) -> Result<CommentDto, UseCaseError> {
    if input.attachments.len() > MAX_COMMENT_ATTACHMENTS {
        return Err(UseCaseError::Invalid(format!(
            "a comment supports at most {MAX_COMMENT_ATTACHMENTS} attachments"
        )));
    }
    let attachments = resolve_attachments(server, &input.attachments)?;
    let display_name = input
        .display_name
        .as_ref()
        .map(|display| resolve_display_name(server, display));
    let dto = {
        let Some(comment) = server.workspace.comments.iter_mut().find(|c| c.id == id) else {
            return Err(UseCaseError::NotFound("comment not found".to_string()));
        };
        comment.rich_text = input.rich_text;
        comment.attachments = attachments;
        comment.property_id = input.property_id;
        comment.display_name = display_name;
        comment.last_edited_time = now_iso();
        comment.clone()
    };
    server
        .write(ChangeOp::Update, "__comment__")
        .map_err(|error| UseCaseError::Storage(error.to_string()))?;
    server.notify(DomainEvent::CommentUpdated {
        comment_id: dto.id.clone(),
        data: serde_json::json!({ "parent": dto.parent }).to_string(),
    });
    Ok(dto)
}

pub(in crate::server) fn delete_comment(
    server: &mut WorkspaceServer,
    id: &str,
) -> Result<CommentDto, UseCaseError> {
    let Some(index) = server.workspace.comments.iter().position(|c| c.id == id) else {
        return Err(UseCaseError::NotFound("comment not found".to_string()));
    };
    let comment = server.workspace.comments.remove(index);
    server
        .write(ChangeOp::Delete, "__comment__")
        .map_err(|error| UseCaseError::Storage(error.to_string()))?;
    server.notify(DomainEvent::CommentDeleted {
        comment_id: comment.id.clone(),
        data: serde_json::json!({ "parent": comment.parent }).to_string(),
    });
    Ok(comment)
}

fn resolve_parent_page(
    server: &WorkspaceServer,
    parent: &ParentDto,
) -> Result<String, UseCaseError> {
    match parent {
        ParentDto::Page { page_id } if server.workspace.page_or_row(page_id).is_some() => {
            Ok(page_id.clone())
        }
        ParentDto::Block { block_id } if server.workspace.block(block_id).is_some() => Ok(server
            .workspace
            .page_id_for_block(block_id)
            .unwrap_or_else(|| "__comment__".to_string())),
        _ => Err(UseCaseError::Invalid(
            "comment parent page or block not found".to_string(),
        )),
    }
}

fn resolve_attachments(
    server: &WorkspaceServer,
    inputs: &[AttachmentInput],
) -> Result<Vec<CommentAttachment>, UseCaseError> {
    inputs
        .iter()
        .map(|input| comment_attachment(server, input).map_err(UseCaseError::Invalid))
        .collect()
}

fn comment_attachment(
    server: &WorkspaceServer,
    input: &AttachmentInput,
) -> Result<CommentAttachment, String> {
    let upload = server
        .workspace
        .file_uploads
        .iter()
        .find(|upload| upload.id == input.file_upload_id)
        .ok_or_else(|| "file upload not found".to_string())?;
    if upload.status != UploadStatus::Uploaded {
        return Err("file upload is not uploaded".to_string());
    }
    Ok(CommentAttachment {
        category: category_for_filename(upload.filename.as_deref()),
        file: FileObject::File {
            url: format!("local://file_uploads/{}", upload.id),
            expiry_time: None,
        },
    })
}

fn resolve_display_name(
    server: &WorkspaceServer,
    display: &CommentDisplayName,
) -> CommentDisplayNameDto {
    match display {
        CommentDisplayName::Connection => CommentDisplayNameDto {
            r#type: "connection".to_string(),
            resolved_name: "Connection".to_string(),
        },
        CommentDisplayName::User => {
            let name = server
                .collab
                .members
                .iter()
                .find(|member| member.id == server.collab.owner_id())
                .map(|member| member.name.clone())
                .unwrap_or_else(|| "User".to_string());
            CommentDisplayNameDto {
                r#type: "user".to_string(),
                resolved_name: name,
            }
        }
        CommentDisplayName::Custom { name } => CommentDisplayNameDto {
            r#type: "custom".to_string(),
            resolved_name: name.clone(),
        },
    }
}

fn category_for_filename(name: Option<&str>) -> String {
    let extension = name
        .and_then(|name| name.rsplit_once('.'))
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "heic" | "bmp" | "tif" | "tiff" => {
            "image"
        }
        "pdf" => "pdf",
        "mp4" | "mov" | "webm" | "mkv" => "video",
        "mp3" | "wav" | "ogg" | "oga" | "m4a" => "audio",
        _ => "productivity",
    }
    .to_string()
}
