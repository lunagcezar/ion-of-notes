//! Comments, users and file-upload use cases (the spec-035 resource family).

use super::super::contract::{
    CODE_INTERNAL, CODE_INVALID_JSON, CODE_NOT_FOUND, CODE_VALIDATION_ERROR,
};
use super::super::pagination::PageRequest;
use super::super::response::{json, parse_body, query_value, v2_error, v2_list_response};
use super::super::{
    ApiRequest, ApiResponse, Capability, Connection, Envelope, Method, WorkspaceServer,
};
use crate::collab::{ChangeOp, Member};
use crate::model::{
    AttachmentInput, CommentAttachment, CommentDisplayName, CommentDisplayNameDto, CommentDto,
    CommentInput, FileObject, FileUpload, FileUploadInput, ParentDto, Person, UploadStatus,
    UserDto, UserRef, UserType, now_iso,
};
use crate::webhooks::WebhookEventType;

impl WorkspaceServer {
    pub(in crate::server) fn handle_v2_comment_user_file(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
        connection: &Connection,
    ) -> Option<ApiResponse> {
        if path == "/api/v2/comments" && request.method == Method::Get {
            let page_filter = query_value(query, "page");
            let block_filter = query_value(query, "block");
            let mut comments: Vec<CommentDto> = self
                .workspace
                .comments
                .iter()
                .filter(|comment| {
                    if !page_filter.is_empty() {
                        return matches!(
                            &comment.parent,
                            ParentDto::Page { page_id } if page_id == &page_filter
                        );
                    }
                    if !block_filter.is_empty() {
                        return matches!(
                            &comment.parent,
                            ParentDto::Block { block_id } if block_id == &block_filter
                        );
                    }
                    true
                })
                .cloned()
                .collect();
            comments.sort_by(|a, b| a.created_time.cmp(&b.created_time));
            let page = match PageRequest::from_query(query) {
                Ok(page) => page,
                Err(error) => {
                    return Some(v2_error(
                        400,
                        error.code(),
                        error.message(),
                        request_id,
                        None,
                    ));
                }
            };
            let sort = format!("comments:{page_filter}:{block_filter}");
            return Some(v2_list_response(
                comments, &page, &sort, "comment", request_id,
            ));
        }
        if path == "/api/v2/comments" && request.method == Method::Post {
            let input = match parse_body::<CommentInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            if input.attachments.len() > 3 {
                return Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    "a comment supports at most 3 attachments",
                    request_id,
                    None,
                ));
            }
            let page_id = match &input.parent {
                ParentDto::Page { page_id } if self.workspace.page_or_row(page_id).is_some() => {
                    page_id.clone()
                }
                ParentDto::Block { block_id } if self.workspace.block(block_id).is_some() => self
                    .workspace
                    .page_id_for_block(block_id)
                    .unwrap_or_else(|| "__comment__".to_string()),
                _ => {
                    return Some(v2_error(
                        400,
                        CODE_VALIDATION_ERROR,
                        "comment parent page or block not found",
                        request_id,
                        None,
                    ));
                }
            };
            let mut attachments = Vec::new();
            for attachment in &input.attachments {
                match self.comment_attachment(attachment) {
                    Ok(attachment) => attachments.push(attachment),
                    Err(message) => {
                        return Some(v2_error(
                            400,
                            CODE_VALIDATION_ERROR,
                            message,
                            request_id,
                            None,
                        ));
                    }
                }
            }
            if input.rich_text.is_empty() && attachments.is_empty() {
                return Some(v2_error(
                    400,
                    CODE_VALIDATION_ERROR,
                    "a comment needs text or an attachment",
                    request_id,
                    None,
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
                created_by: UserRef::new(self.collab.owner_id()),
                rich_text: input.rich_text,
                attachments,
                display_name: input
                    .display_name
                    .as_ref()
                    .map(|display| self.resolve_display_name(display)),
                has_pending_attachments: false,
            };
            self.workspace.comments.push(comment.clone());
            if let Err(error) = self.write(ChangeOp::Create, &page_id) {
                return Some(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            self.emit_webhook(
                WebhookEventType::CommentCreated,
                "comment",
                &comment.id,
                serde_json::json!({ "page_id": page_id.clone(), "parent": comment.parent.clone() })
                    .to_string(),
            );
            return Some(json(201, Envelope::ok(comment)));
        }

        if let Some(id) = path.strip_prefix("/api/v2/comments/") {
            if id.is_empty() || id.contains('/') {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "comment not found",
                    request_id,
                    None,
                ));
            }
            match request.method {
                Method::Get => match self.workspace.comments.iter().find(|c| c.id == id) {
                    Some(comment) => Some(json(200, Envelope::ok(comment.clone()))),
                    None => Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "comment not found",
                        request_id,
                        None,
                    )),
                },
                Method::Patch => {
                    let input = match parse_body::<CommentInput>(request) {
                        Ok(input) => input,
                        Err(message) => {
                            return Some(v2_error(
                                400,
                                CODE_INVALID_JSON,
                                message,
                                request_id,
                                None,
                            ));
                        }
                    };
                    if input.attachments.len() > 3 {
                        return Some(v2_error(
                            400,
                            CODE_VALIDATION_ERROR,
                            "a comment supports at most 3 attachments",
                            request_id,
                            None,
                        ));
                    }
                    let mut attachments = Vec::new();
                    for attachment in &input.attachments {
                        match self.comment_attachment(attachment) {
                            Ok(attachment) => attachments.push(attachment),
                            Err(message) => {
                                return Some(v2_error(
                                    400,
                                    CODE_VALIDATION_ERROR,
                                    message,
                                    request_id,
                                    None,
                                ));
                            }
                        }
                    }
                    let display_name = input
                        .display_name
                        .as_ref()
                        .map(|display| self.resolve_display_name(display));
                    let dto = {
                        let Some(comment) = self.workspace.comments.iter_mut().find(|c| c.id == id)
                        else {
                            return Some(v2_error(
                                404,
                                CODE_NOT_FOUND,
                                "comment not found",
                                request_id,
                                None,
                            ));
                        };
                        comment.rich_text = input.rich_text;
                        comment.attachments = attachments;
                        comment.property_id = input.property_id;
                        comment.display_name = display_name;
                        comment.last_edited_time = now_iso();
                        comment.clone()
                    };
                    if let Err(error) = self.write(ChangeOp::Update, "__comment__") {
                        return Some(v2_error(
                            500,
                            CODE_INTERNAL,
                            error.to_string(),
                            request_id,
                            None,
                        ));
                    }
                    self.emit_webhook(
                        WebhookEventType::CommentUpdated,
                        "comment",
                        &dto.id,
                        serde_json::json!({ "parent": dto.parent.clone() }).to_string(),
                    );
                    Some(json(200, Envelope::ok(dto)))
                }
                Method::Delete => {
                    let Some(index) = self.workspace.comments.iter().position(|c| c.id == id)
                    else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "comment not found",
                            request_id,
                            None,
                        ));
                    };
                    let comment = self.workspace.comments.remove(index);
                    if let Err(error) = self.write(ChangeOp::Delete, "__comment__") {
                        return Some(v2_error(
                            500,
                            CODE_INTERNAL,
                            error.to_string(),
                            request_id,
                            None,
                        ));
                    }
                    self.emit_webhook(
                        WebhookEventType::CommentDeleted,
                        "comment",
                        &comment.id,
                        serde_json::json!({ "parent": comment.parent.clone() }).to_string(),
                    );
                    Some(json(200, Envelope::ok(comment)))
                }
                _ => Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "unknown route",
                    request_id,
                    None,
                )),
            }
        } else if path == "/api/v2/users" && request.method == Method::Get {
            let level = user_info_level(connection);
            let mut users: Vec<UserDto> = self
                .collab
                .members
                .iter()
                .map(|member| user_dto(member, level))
                .collect();
            users.push(UserDto::bot(
                connection.id.clone(),
                connection.workspace_id.clone(),
                Some(self.workspace.name.clone()),
            ));
            Some(json(200, Envelope::ok(users)))
        } else if let Some(id) = path.strip_prefix("/api/v2/users/") {
            if request.method != Method::Get || id.is_empty() || id.contains('/') {
                return Some(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "user not found",
                    request_id,
                    None,
                ));
            }
            if let Some(member) = self.collab.members.iter().find(|member| member.id == id) {
                return Some(json(
                    200,
                    Envelope::ok(user_dto(member, user_info_level(connection))),
                ));
            }
            if id == connection.id {
                return Some(json(
                    200,
                    Envelope::ok(UserDto::bot(
                        connection.id.clone(),
                        connection.workspace_id.clone(),
                        Some(self.workspace.name.clone()),
                    )),
                ));
            }
            Some(v2_error(
                404,
                CODE_NOT_FOUND,
                "user not found",
                request_id,
                None,
            ))
        } else if path == "/api/v2/file_uploads" && request.method == Method::Get {
            let page = match PageRequest::from_query(query) {
                Ok(page) => page,
                Err(error) => {
                    return Some(v2_error(
                        400,
                        error.code(),
                        error.message(),
                        request_id,
                        None,
                    ));
                }
            };
            Some(v2_list_response(
                self.workspace.file_uploads.clone(),
                &page,
                "file_uploads",
                "file_upload",
                request_id,
            ))
        } else if path == "/api/v2/file_uploads" && request.method == Method::Post {
            let input = match parse_body::<FileUploadInput>(request) {
                Ok(input) => input,
                Err(message) => {
                    return Some(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
                }
            };
            let now = now_iso();
            let id = format!("upload-{}", uuid::Uuid::new_v4());
            let upload = FileUpload {
                object: "file_upload".to_string(),
                id: id.clone(),
                created_time: now.clone(),
                last_edited_time: now,
                expiry_time: Some((chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339()),
                status: UploadStatus::Pending,
                filename: input.filename,
                content_type: input.content_type,
                content_length: input.content_length,
                upload_url: Some(format!("local://file_uploads/{id}/send")),
                complete_url: None,
            };
            self.workspace.file_uploads.push(upload.clone());
            if let Err(error) = self.write(ChangeOp::Create, "__file_upload__") {
                return Some(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            self.emit_webhook(
                WebhookEventType::FileUploadCreated,
                "file_upload",
                &upload.id,
                serde_json::json!({ "status": "pending" }).to_string(),
            );
            Some(json(201, Envelope::ok(upload)))
        } else if let Some(rest) = path.strip_prefix("/api/v2/file_uploads/") {
            if let Some(id) = rest.strip_suffix("/send") {
                if request.method != Method::Post {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "unknown route",
                        request_id,
                        None,
                    ));
                }
                let Some(content) = request.body.as_ref() else {
                    return Some(v2_error(
                        400,
                        CODE_VALIDATION_ERROR,
                        "file content is required",
                        request_id,
                        None,
                    ));
                };
                let dto = {
                    let Some(upload) = self
                        .workspace
                        .file_uploads
                        .iter_mut()
                        .find(|upload| upload.id == id)
                    else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "file upload not found",
                            request_id,
                            None,
                        ));
                    };
                    upload.status = UploadStatus::Uploaded;
                    upload.content_length = Some(content.len() as u64);
                    upload.upload_url = None;
                    upload.expiry_time = None;
                    upload.last_edited_time = now_iso();
                    upload.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__file_upload__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.emit_webhook(
                    WebhookEventType::FileUploadCompleted,
                    "file_upload",
                    &dto.id,
                    serde_json::json!({ "status": "uploaded" }).to_string(),
                );
                return Some(json(200, Envelope::ok(dto)));
            }
            if let Some(id) = rest.strip_suffix("/complete") {
                if request.method != Method::Post {
                    return Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "unknown route",
                        request_id,
                        None,
                    ));
                }
                let dto = {
                    let Some(upload) = self
                        .workspace
                        .file_uploads
                        .iter_mut()
                        .find(|upload| upload.id == id)
                    else {
                        return Some(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "file upload not found",
                            request_id,
                            None,
                        ));
                    };
                    if upload.status != UploadStatus::Pending {
                        return Some(v2_error(
                            400,
                            CODE_VALIDATION_ERROR,
                            "file upload is not pending",
                            request_id,
                            None,
                        ));
                    }
                    upload.status = UploadStatus::Uploaded;
                    upload.upload_url = None;
                    upload.complete_url = None;
                    upload.expiry_time = None;
                    upload.last_edited_time = now_iso();
                    upload.clone()
                };
                if let Err(error) = self.write(ChangeOp::Update, "__file_upload__") {
                    return Some(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.emit_webhook(
                    WebhookEventType::FileUploadCompleted,
                    "file_upload",
                    &dto.id,
                    serde_json::json!({ "status": "uploaded" }).to_string(),
                );
                return Some(json(200, Envelope::ok(dto)));
            }
            if request.method == Method::Get && !rest.is_empty() && !rest.contains('/') {
                return match self
                    .workspace
                    .file_uploads
                    .iter()
                    .find(|upload| upload.id == rest)
                {
                    Some(upload) => Some(json(200, Envelope::ok(upload.clone()))),
                    None => Some(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "file upload not found",
                        request_id,
                        None,
                    )),
                };
            }
            None
        } else {
            None
        }
    }

    fn comment_attachment(&self, input: &AttachmentInput) -> Result<CommentAttachment, String> {
        let upload = self
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

    fn resolve_display_name(&self, display: &CommentDisplayName) -> CommentDisplayNameDto {
        match display {
            CommentDisplayName::Connection => CommentDisplayNameDto {
                r#type: "connection".to_string(),
                resolved_name: "Connection".to_string(),
            },
            CommentDisplayName::User => {
                let name = self
                    .collab
                    .members
                    .iter()
                    .find(|member| member.id == self.collab.owner_id())
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
}

fn user_info_level(connection: &Connection) -> u8 {
    if connection.has(Capability::UserInfoWithEmail) {
        2
    } else if connection.has(Capability::UserInfoWithoutEmail) {
        1
    } else {
        0
    }
}

fn user_dto(member: &Member, level: u8) -> UserDto {
    if level == 0 {
        return UserDto {
            object: "user".to_string(),
            id: member.id.clone(),
            r#type: None,
            name: None,
            avatar_url: None,
            person: None,
            bot: None,
        };
    }
    UserDto {
        object: "user".to_string(),
        id: member.id.clone(),
        r#type: Some(UserType::Person),
        name: Some(member.name.clone()),
        avatar_url: None,
        person: Some(Person {
            email: (level >= 2).then(|| member.email.clone()),
            email_verified: level >= 2,
        }),
        bot: None,
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
