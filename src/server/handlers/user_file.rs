//! Users and file-upload use cases (the spec-035 resource family).

use super::super::contract::{
    CODE_INTERNAL, CODE_INVALID_JSON, CODE_NOT_FOUND, CODE_VALIDATION_ERROR,
};
use super::super::error::{HandlerResult, fail, ok};
use super::super::events::DomainEvent;
use super::super::pagination::PageRequest;
use super::super::response::{json, parse_body, v2_error, v2_list_response};
use super::super::{
    ApiRequest, ApiResponse, Capability, Connection, Envelope, Method, WorkspaceServer,
};
use crate::collab::{ChangeOp, Member};
use crate::model::{FileUpload, FileUploadInput, Person, UploadStatus, UserDto, UserType, now_iso};

impl WorkspaceServer {
    pub(in crate::server) fn handle_v2_user_file(
        &mut self,
        request: &ApiRequest,
        path: &str,
        query: &str,
        request_id: &str,
        connection: &Connection,
    ) -> HandlerResult<Option<ApiResponse>> {
        if path == "/api/v2/users" && request.method == Method::Get {
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
            ok(json(200, Envelope::ok(users)))
        } else if let Some(id) = path.strip_prefix("/api/v2/users/") {
            if request.method != Method::Get || id.is_empty() || id.contains('/') {
                return fail(v2_error(
                    404,
                    CODE_NOT_FOUND,
                    "user not found",
                    request_id,
                    None,
                ));
            }
            if let Some(member) = self.collab.members.iter().find(|member| member.id == id) {
                return ok(json(
                    200,
                    Envelope::ok(user_dto(member, user_info_level(connection))),
                ));
            }
            if id == connection.id {
                return ok(json(
                    200,
                    Envelope::ok(UserDto::bot(
                        connection.id.clone(),
                        connection.workspace_id.clone(),
                        Some(self.workspace.name.clone()),
                    )),
                ));
            }
            fail(v2_error(
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
                    return fail(v2_error(
                        400,
                        error.code(),
                        error.message(),
                        request_id,
                        None,
                    ));
                }
            };
            ok(v2_list_response(
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
                    return fail(v2_error(400, CODE_INVALID_JSON, message, request_id, None));
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
                return fail(v2_error(
                    500,
                    CODE_INTERNAL,
                    error.to_string(),
                    request_id,
                    None,
                ));
            }
            self.notify(DomainEvent::FileUploadCreated {
                upload_id: upload.id.clone(),
            });
            ok(json(201, Envelope::ok(upload)))
        } else if let Some(rest) = path.strip_prefix("/api/v2/file_uploads/") {
            if let Some(id) = rest.strip_suffix("/send") {
                if request.method != Method::Post {
                    return fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "unknown route",
                        request_id,
                        None,
                    ));
                }
                let Some(content) = request.body.as_ref() else {
                    return fail(v2_error(
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
                        return fail(v2_error(
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
                    return fail(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.notify(DomainEvent::FileUploadCompleted {
                    upload_id: dto.id.clone(),
                });
                return ok(json(200, Envelope::ok(dto)));
            }
            if let Some(id) = rest.strip_suffix("/complete") {
                if request.method != Method::Post {
                    return fail(v2_error(
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
                        return fail(v2_error(
                            404,
                            CODE_NOT_FOUND,
                            "file upload not found",
                            request_id,
                            None,
                        ));
                    };
                    if upload.status != UploadStatus::Pending {
                        return fail(v2_error(
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
                    return fail(v2_error(
                        500,
                        CODE_INTERNAL,
                        error.to_string(),
                        request_id,
                        None,
                    ));
                }
                self.notify(DomainEvent::FileUploadCompleted {
                    upload_id: dto.id.clone(),
                });
                return ok(json(200, Envelope::ok(dto)));
            }
            if request.method == Method::Get && !rest.is_empty() && !rest.contains('/') {
                return match self
                    .workspace
                    .file_uploads
                    .iter()
                    .find(|upload| upload.id == rest)
                {
                    Some(upload) => ok(json(200, Envelope::ok(upload.clone()))),
                    None => fail(v2_error(
                        404,
                        CODE_NOT_FOUND,
                        "file upload not found",
                        request_id,
                        None,
                    )),
                };
            }
            Ok(None)
        } else {
            Ok(None)
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
