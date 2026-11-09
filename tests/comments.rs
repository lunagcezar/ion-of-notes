//! Comments, users and file-upload tests (spec-035).

mod common;

use common::{data, list_data, v2_request, v2_request_as};
use not_notion::client::Client;
use not_notion::model::{
    AttachmentInput, CommentDisplayName, CommentDto, CommentInput, FileObject, FileUpload,
    FileUploadInput, ParentDto, RichText, UploadStatus, UserDto, UserType, Workspace,
};
use not_notion::server::{
    Capability, Connection, ListResponse, Method, WorkspaceApi, WorkspaceServer,
};

fn upload(server: &mut WorkspaceServer, filename: &str) -> String {
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/file_uploads",
        Some(
            serde_json::to_string(&FileUploadInput {
                filename: Some(filename.to_string()),
                content_type: Some("image/png".to_string()),
                content_length: None,
            })
            .unwrap(),
        ),
    ));
    assert_eq!(response.status, 201);
    let upload: FileUpload = data(&response);
    assert_eq!(upload.status, UploadStatus::Pending);

    let response = server.handle(&v2_request(
        Method::Post,
        &format!("/api/v2/file_uploads/{}/send", upload.id),
        Some("file bytes".to_string()),
    ));
    assert_eq!(response.status, 200);
    let upload: FileUpload = data(&response);
    assert_eq!(upload.status, UploadStatus::Uploaded);
    upload.id
}

fn comment_input(parent: ParentDto, text: &str) -> CommentInput {
    CommentInput {
        parent,
        discussion_id: None,
        property_id: None,
        rich_text: vec![RichText::text(text)],
        attachments: Vec::new(),
        display_name: Some(CommentDisplayName::Custom {
            name: "Test bot".to_string(),
        }),
    }
}

#[test]
fn comment_roundtrip_with_attachment() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let upload_id = upload(&mut server, "note.png");
    let mut input = comment_input(
        ParentDto::Page {
            page_id: "p-home".to_string(),
        },
        "Hello comment",
    );
    input.attachments = vec![AttachmentInput {
        file_upload_id: upload_id,
    }];

    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/comments",
        Some(serde_json::to_string(&input).unwrap()),
    ));
    assert_eq!(response.status, 201);
    let comment: CommentDto = data(&response);
    assert_eq!(comment.object, "comment");
    assert_eq!(
        comment.parent,
        ParentDto::Page {
            page_id: "p-home".to_string()
        }
    );
    assert_eq!(comment.attachments.len(), 1);
    assert_eq!(comment.attachments[0].category, "image");
    assert!(matches!(
        comment.attachments[0].file,
        FileObject::File { .. }
    ));
    assert_eq!(
        comment
            .display_name
            .as_ref()
            .map(|name| name.resolved_name.as_str()),
        Some("Test bot")
    );

    let response = server.handle(&v2_request(
        Method::Get,
        &format!("/api/v2/comments/{}", comment.id),
        None,
    ));
    assert_eq!(response.status, 200);
    let fetched: CommentDto = data(&response);
    assert_eq!(fetched.id, comment.id);
}

#[test]
fn comment_can_target_a_block() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let input = comment_input(
        ParentDto::Block {
            block_id: "h1".to_string(),
        },
        "Block comment",
    );
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/comments",
        Some(serde_json::to_string(&input).unwrap()),
    ));
    assert_eq!(response.status, 201);
    let comment: CommentDto = data(&response);
    assert_eq!(
        comment.parent,
        ParentDto::Block {
            block_id: "h1".to_string()
        }
    );
}

#[test]
fn comments_are_paginated_and_deletable() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    for text in ["First", "Second"] {
        let input = comment_input(
            ParentDto::Page {
                page_id: "p-home".to_string(),
            },
            text,
        );
        let response = server.handle(&v2_request(
            Method::Post,
            "/api/v2/comments",
            Some(serde_json::to_string(&input).unwrap()),
        ));
        assert_eq!(response.status, 201);
    }

    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/comments?page=p-home&page_size=1",
        None,
    ));
    assert_eq!(response.status, 200);
    let first: ListResponse<CommentDto> = list_data(&response);
    assert_eq!(first.results.len(), 1);
    assert!(first.has_more);

    let response = server.handle(&v2_request(
        Method::Get,
        &format!(
            "/api/v2/comments?page=p-home&page_size=1&start_cursor={}",
            first.next_cursor.unwrap()
        ),
        None,
    ));
    let second: ListResponse<CommentDto> = list_data(&response);
    assert_eq!(second.results.len(), 1);

    let id = first.results[0].id.clone();
    let response = server.handle(&v2_request(
        Method::Delete,
        &format!("/api/v2/comments/{id}"),
        None,
    ));
    assert_eq!(response.status, 200);
    let response = server.handle(&v2_request(
        Method::Get,
        &format!("/api/v2/comments/{id}"),
        None,
    ));
    assert_eq!(response.status, 404);
}

#[test]
fn comment_rejects_pending_attachments_and_too_many() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/file_uploads",
        Some(
            serde_json::to_string(&FileUploadInput {
                filename: Some("pending.txt".to_string()),
                content_type: None,
                content_length: None,
            })
            .unwrap(),
        ),
    ));
    let pending: FileUpload = data(&response);
    let mut input = comment_input(
        ParentDto::Page {
            page_id: "p-home".to_string(),
        },
        "Pending",
    );
    input.attachments = vec![AttachmentInput {
        file_upload_id: pending.id,
    }];
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/comments",
        Some(serde_json::to_string(&input).unwrap()),
    ));
    assert_eq!(response.status, 400);

    let upload_id = upload(&mut server, "a.txt");
    input.attachments = (0..4)
        .map(|_| AttachmentInput {
            file_upload_id: upload_id.clone(),
        })
        .collect();
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/comments",
        Some(serde_json::to_string(&input).unwrap()),
    ));
    assert_eq!(response.status, 400);
}

#[test]
fn comment_requires_a_valid_parent() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let input = comment_input(
        ParentDto::Page {
            page_id: "missing".to_string(),
        },
        "Orphan",
    );
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/comments",
        Some(serde_json::to_string(&input).unwrap()),
    ));
    assert_eq!(response.status, 400);
}

#[test]
fn users_respect_user_info_capabilities() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(Method::Get, "/api/v2/users", None));
    let users: Vec<UserDto> = data(&response);
    let owner = users.iter().find(|user| user.id == "m-owner").unwrap();
    assert_eq!(owner.r#type, Some(UserType::Person));
    assert_eq!(
        owner
            .person
            .as_ref()
            .and_then(|person| person.email.as_deref()),
        Some("owner@workspace.local")
    );
    assert!(users.iter().any(|user| user.r#type == Some(UserType::Bot)));

    server.register_connection(Connection::new(
        "no-info",
        "no-info-token",
        "m-owner",
        "local-workspace",
        [Capability::ReadContent, Capability::UserInfoNone],
    ));
    let response = server.handle(&v2_request_as(
        "no-info-token",
        Method::Get,
        "/api/v2/users",
        None,
    ));
    let users: Vec<UserDto> = data(&response);
    let owner = users.iter().find(|user| user.id == "m-owner").unwrap();
    assert_eq!(owner.name, None);
    assert!(owner.person.is_none());

    server.register_connection(Connection::new(
        "no-email",
        "no-email-token",
        "m-owner",
        "local-workspace",
        [Capability::ReadContent, Capability::UserInfoWithoutEmail],
    ));
    let response = server.handle(&v2_request_as(
        "no-email-token",
        Method::Get,
        "/api/v2/users",
        None,
    ));
    let users: Vec<UserDto> = data(&response);
    let owner = users.iter().find(|user| user.id == "m-owner").unwrap();
    assert_eq!(owner.name.as_deref(), Some("Workspace Owner"));
    assert_eq!(
        owner
            .person
            .as_ref()
            .and_then(|person| person.email.as_deref()),
        None
    );
}

#[test]
fn file_upload_lifecycle_and_listing() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/file_uploads",
        Some(
            serde_json::to_string(&FileUploadInput {
                filename: Some("doc.pdf".to_string()),
                content_type: Some("application/pdf".to_string()),
                content_length: None,
            })
            .unwrap(),
        ),
    ));
    let pending: FileUpload = data(&response);
    assert_eq!(pending.status, UploadStatus::Pending);
    assert!(pending.upload_url.is_some());

    let response = server.handle(&v2_request(
        Method::Post,
        &format!("/api/v2/file_uploads/{}/send", pending.id),
        Some("pdf bytes".to_string()),
    ));
    let uploaded: FileUpload = data(&response);
    assert_eq!(uploaded.status, UploadStatus::Uploaded);
    assert!(uploaded.upload_url.is_none());
    assert_eq!(uploaded.content_length, Some(9));

    let response = server.handle(&v2_request(
        Method::Post,
        &format!("/api/v2/file_uploads/{}/complete", pending.id),
        None,
    ));
    assert_eq!(response.status, 400);

    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/file_uploads?page_size=1",
        None,
    ));
    let list: ListResponse<FileUpload> = list_data(&response);
    assert_eq!(list.results.len(), 1);
    assert!(!list.has_more);
}

#[test]
fn client_comment_and_upload_roundtrip() {
    let mut client = Client::with_token(WorkspaceServer::new(Workspace::seed("x")), "owner-token");
    let upload = client
        .create_file_upload_v2(&FileUploadInput {
            filename: Some("client.txt".to_string()),
            content_type: Some("text/plain".to_string()),
            content_length: None,
        })
        .unwrap();
    let uploaded = client.send_file_upload_v2(&upload.id, "hello").unwrap();
    assert_eq!(uploaded.status, UploadStatus::Uploaded);

    let input = CommentInput {
        parent: ParentDto::Page {
            page_id: "p-home".to_string(),
        },
        discussion_id: None,
        property_id: None,
        rich_text: vec![RichText::text("Client comment")],
        attachments: vec![AttachmentInput {
            file_upload_id: uploaded.id,
        }],
        display_name: None,
    };
    let comment = client.create_comment_v2(&input).unwrap();
    assert_eq!(client.get_comment_v2(&comment.id).unwrap().id, comment.id);
    assert_eq!(
        client.list_comments_v2(Some("p-home"), None).unwrap().len(),
        1
    );
    client.delete_comment_v2(&comment.id).unwrap();
}
