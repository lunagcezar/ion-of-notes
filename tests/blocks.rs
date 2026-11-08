//! Rich text, block metadata, children and block endpoint tests (spec-032).

mod common;

use common::{data, list_data, v2_request};
use not_notion::client::Client;
use not_notion::model::{
    Annotations, Block, BlockDto, BlockInput, BlockKind, Mention, ParentDto, RichText, TextContent,
    TextLink, Workspace,
};
use not_notion::server::{ListResponse, Method, WorkspaceApi, WorkspaceServer};

fn rich_text_with_bold(text: &str) -> RichText {
    let annotations = Annotations {
        bold: true,
        ..Annotations::default()
    };
    RichText::Text {
        text: TextContent {
            content: text.to_string(),
            link: None,
        },
        annotations,
        plain_text: text.to_string(),
        href: None,
    }
}

#[test]
fn rich_text_roundtrips_text_link_equation_and_mention() {
    let text = RichText::Text {
        text: TextContent {
            content: "Hello".to_string(),
            link: Some(TextLink {
                url: "https://example.com".to_string(),
            }),
        },
        annotations: Annotations {
            bold: true,
            italic: true,
            ..Annotations::default()
        },
        plain_text: "Hello".to_string(),
        href: Some("https://example.com".to_string()),
    };
    let json = serde_json::to_string(&text).unwrap();
    let back: RichText = serde_json::from_str(&json).unwrap();
    assert_eq!(back, text);
    assert_eq!(back.plain_text(), "Hello");

    let equation = RichText::equation("e=mc^2");
    assert_eq!(
        serde_json::from_str::<RichText>(&serde_json::to_string(&equation).unwrap()).unwrap(),
        equation
    );

    let mention = RichText::Mention {
        mention: Mention::Page {
            id: "p-home".to_string(),
        },
        annotations: Annotations::default(),
        plain_text: "Home".to_string(),
        href: None,
    };
    assert_eq!(
        serde_json::from_str::<RichText>(&serde_json::to_string(&mention).unwrap()).unwrap(),
        mention
    );
}

#[test]
fn block_metadata_and_dto_are_populated() {
    let block = Block::new("b1", BlockKind::Paragraph, "Hello");
    assert_eq!(block.object, "block");
    assert_eq!(block.plain_text(), "Hello");
    assert!(!block.has_children());

    let dto = block.to_dto("p-home");
    assert!(!dto.created_time.is_empty());
    assert!(!dto.last_edited_time.is_empty());
    assert_eq!(dto.object, "block");
    assert_eq!(dto.id, "b1");
    assert_eq!(
        dto.parent,
        ParentDto::Page {
            page_id: "p-home".to_string()
        }
    );
    assert_eq!(dto.r#type, "paragraph");
    assert_eq!(dto.rich_text, vec![RichText::text("Hello")]);
}

#[test]
fn legacy_text_is_migrated_to_rich_text() {
    let json =
        r#"{"id":"legacy","kind":"Paragraph","text":"old text","checked":false,"collapsed":false}"#;
    let mut block: Block = serde_json::from_str(json).unwrap();
    assert!(block.rich_text.is_empty());
    block.normalize();
    assert_eq!(block.rich_text.len(), 1);
    assert_eq!(block.plain_text(), "old text");
}

#[test]
fn editing_legacy_text_updates_effective_rich_text() {
    let mut block = Block::new("b1", BlockKind::Paragraph, "old");
    block.text = "new".to_string();
    assert_eq!(block.plain_text(), "new");
    let dto = block.to_dto("p-home");
    assert_eq!(dto.rich_text[0].plain_text(), "new");
}

#[test]
fn nested_children_are_represented() {
    let mut parent = Block::new("parent", BlockKind::Toggle, "Toggle");
    let mut child = Block::new("child", BlockKind::Paragraph, "Child");
    child.parent_block = Some("parent".to_string());
    parent.children.push(child);

    assert!(parent.has_children());
    let dto = parent.to_dto("p-home");
    assert!(dto.has_children);
    assert_eq!(dto.collapsed, Some(false));
}

#[test]
fn table_metadata_and_cells_are_preserved() {
    let mut table = Block::new("table", BlockKind::Table, "");
    table.table_width = Some(2);
    table.has_column_header = true;
    let dto = table.to_dto("p-home");
    assert_eq!(dto.r#type, "table");
    assert_eq!(dto.table_width, Some(2));
    assert_eq!(dto.has_column_header, Some(true));

    let mut row = Block::new("row", BlockKind::TableRow, "");
    row.cells = vec![vec![RichText::text("A")], vec![RichText::text("B")]];
    let dto = row.to_dto("p-home");
    assert_eq!(dto.r#type, "table_row");
    assert_eq!(dto.cells.unwrap().len(), 2);
}

#[test]
fn unsupported_block_types_are_explicit() {
    let block = Block::from_input(
        BlockInput {
            r#type: "mystery_block".to_string(),
            ..BlockInput::default()
        },
        "mystery".to_string(),
        None,
        Some("m-owner".to_string()),
    );
    assert_eq!(block.kind, BlockKind::Unsupported);
    assert_eq!(block.unsupported_type.as_deref(), Some("mystery_block"));
    let dto = block.to_dto("p-home");
    assert_eq!(dto.r#type, "mystery_block");
    assert_eq!(dto.unsupported_type.as_deref(), Some("mystery_block"));
}

#[test]
fn v2_get_block_and_children() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(Method::Get, "/api/v2/blocks/h1", None));
    assert_eq!(response.status, 200);
    let block: BlockDto = data(&response);
    assert_eq!(block.id, "h1");
    assert_eq!(
        block.parent,
        ParentDto::Page {
            page_id: "p-home".to_string()
        }
    );
    assert_eq!(block.r#type, "callout");

    let home = server.workspace.page("p-home").unwrap();
    let total = home.blocks.len();
    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/blocks/p-home/children?page_size=2",
        None,
    ));
    assert_eq!(response.status, 200);
    let first: ListResponse<BlockDto> = list_data(&response);
    assert_eq!(first.results.len(), 2);
    assert!(first.has_more);

    let response = server.handle(&v2_request(
        Method::Get,
        &format!(
            "/api/v2/blocks/p-home/children?page_size=2&start_cursor={}",
            first.next_cursor.unwrap()
        ),
        None,
    ));
    let second: ListResponse<BlockDto> = list_data(&response);
    assert!(second.results.len() <= total - 2);
}

#[test]
fn v2_append_update_and_trash_block() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(
        Method::Patch,
        "/api/v2/blocks/p-home/children",
        Some(
            serde_json::json!({
                "children": [{
                    "type": "paragraph",
                    "rich_text": [{
                        "type": "text",
                        "text": { "content": "New block", "link": null },
                        "annotations": {
                            "bold": false, "italic": false, "strikethrough": false,
                            "underline": false, "code": false, "color": "default"
                        },
                        "plain_text": "New block"
                    }]
                }]
            })
            .to_string(),
        ),
    ));
    assert_eq!(response.status, 200);
    let created: Vec<BlockDto> = data(&response);
    assert_eq!(created.len(), 1);
    let id = created[0].id.clone();
    assert_eq!(created[0].r#type, "paragraph");

    let update = BlockInput {
        r#type: "paragraph".to_string(),
        rich_text: vec![rich_text_with_bold("Updated")],
        ..BlockInput::default()
    };
    let response = server.handle(&v2_request(
        Method::Patch,
        &format!("/api/v2/blocks/{id}"),
        Some(serde_json::to_string(&update).unwrap()),
    ));
    assert_eq!(response.status, 200);
    let updated: BlockDto = data(&response);
    assert_eq!(updated.rich_text[0].plain_text(), "Updated");
    match &updated.rich_text[0] {
        RichText::Text { annotations, .. } => assert!(annotations.bold),
        other => panic!("expected text rich text, got {other:?}"),
    }

    let response = server.handle(&v2_request(
        Method::Delete,
        &format!("/api/v2/blocks/{id}"),
        None,
    ));
    assert_eq!(response.status, 200);
    let deleted: BlockDto = data(&response);
    assert!(deleted.in_trash);
}

#[test]
fn v2_nested_append_sets_block_parent() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(
        Method::Patch,
        "/api/v2/blocks/h1/children",
        Some(
            serde_json::json!({
                "children": [{
                    "type": "paragraph",
                    "rich_text": [{
                        "type": "text",
                        "text": { "content": "Nested", "link": null },
                        "annotations": {
                            "bold": false, "italic": false, "strikethrough": false,
                            "underline": false, "code": false, "color": "default"
                        },
                        "plain_text": "Nested"
                    }]
                }]
            })
            .to_string(),
        ),
    ));
    assert_eq!(response.status, 200);
    let created: Vec<BlockDto> = data(&response);
    assert_eq!(
        created[0].parent,
        ParentDto::Block {
            block_id: "h1".to_string()
        }
    );

    let response = server.handle(&v2_request(Method::Get, "/api/v2/blocks/h1", None));
    let parent: BlockDto = data(&response);
    assert!(parent.has_children);
}

#[test]
fn v2_block_errors_are_structured() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(
        Method::Patch,
        "/api/v2/blocks/nope/children",
        Some("{\"children\":[]}".to_string()),
    ));
    assert_eq!(response.status, 404);
    let error: not_notion::ApiError = serde_json::from_str(&response.body).unwrap();
    assert_eq!(error.code, "object_not_found");

    let response = server.handle(&v2_request(Method::Get, "/api/v2/blocks/nope", None));
    assert_eq!(response.status, 404);
}

#[test]
fn client_block_roundtrip() {
    let server = WorkspaceServer::new(Workspace::seed("x"));
    let mut client = Client::with_token(server, "owner-token");
    let block = client.get_block_v2("h1").unwrap();
    assert_eq!(block.id, "h1");

    let children = client.list_block_children_v2("p-home").unwrap();
    assert!(!children.is_empty());

    let created = client
        .append_block_children_v2(
            "p-home",
            &[BlockInput {
                r#type: "paragraph".to_string(),
                text: Some("Client block".to_string()),
                ..BlockInput::default()
            }],
        )
        .unwrap();
    assert_eq!(created.len(), 1);
    assert_eq!(
        client.get_block_v2(&created[0].id).unwrap().r#type,
        "paragraph"
    );
}
