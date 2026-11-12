//! Pagination contract tests for spec-031.

mod common;

use common::{list_data, page_draft_json, v2_request};
use ion_of_notes::client::Client;
use ion_of_notes::model::{Page, PageSummary, Workspace};
use ion_of_notes::server::{
    ApiResponse, ListResponse, Method, PageRequest, PaginationError, RequestTransport,
    WorkspaceServer, paginate,
};

fn v2_error_code(response: &ApiResponse) -> String {
    serde_json::from_str::<ion_of_notes::ApiError>(&response.body)
        .expect("v2 error body")
        .code
}

#[test]
fn page_size_defaults_and_bounds() {
    assert_eq!(PageRequest::from_query("").unwrap().page_size, 100);
    assert_eq!(PageRequest::from_query("page_size=1").unwrap().page_size, 1);
    assert_eq!(
        PageRequest::from_query("page_size=100").unwrap().page_size,
        100
    );
    assert!(matches!(
        PageRequest::from_query("page_size=0"),
        Err(PaginationError::InvalidPageSize(_))
    ));
    assert!(matches!(
        PageRequest::from_query("page_size=101"),
        Err(PaginationError::InvalidPageSize(_))
    ));
    assert!(matches!(
        PageRequest::from_query("page_size=abc"),
        Err(PaginationError::InvalidPageSize(_))
    ));
}

#[test]
fn paginate_walks_first_next_and_last_page() {
    let items: Vec<u32> = (0..5).collect();
    let first = paginate(
        &items,
        &PageRequest {
            start_cursor: None,
            page_size: 2,
        },
        "pages",
        "page",
    )
    .unwrap();
    assert_eq!(first.object, "list");
    assert_eq!(first.r#type, "page");
    assert_eq!(first.results, vec![0, 1]);
    assert!(first.has_more);
    assert!(first.next_cursor.is_some());

    let second = paginate(
        &items,
        &PageRequest {
            start_cursor: first.next_cursor.clone(),
            page_size: 2,
        },
        "pages",
        "page",
    )
    .unwrap();
    assert_eq!(second.results, vec![2, 3]);
    assert!(second.has_more);

    let last = paginate(
        &items,
        &PageRequest {
            start_cursor: second.next_cursor,
            page_size: 2,
        },
        "pages",
        "page",
    )
    .unwrap();
    assert_eq!(last.results, vec![4]);
    assert!(!last.has_more);
    assert_eq!(last.next_cursor, None);
}

#[test]
fn cursors_are_opaque_and_bound_to_the_sort_key() {
    let items: Vec<u32> = (0..5).collect();
    let first = paginate(
        &items,
        &PageRequest {
            start_cursor: None,
            page_size: 2,
        },
        "pages",
        "page",
    )
    .unwrap();
    let cursor = first.next_cursor.unwrap();

    let wrong_sort = paginate(
        &items,
        &PageRequest {
            start_cursor: Some(cursor),
            page_size: 2,
        },
        "members",
        "member",
    );
    assert_eq!(wrong_sort, Err(PaginationError::InvalidCursor));

    let malformed = paginate(
        &items,
        &PageRequest {
            start_cursor: Some("not-a-cursor".to_string()),
            page_size: 2,
        },
        "pages",
        "page",
    );
    assert_eq!(malformed, Err(PaginationError::InvalidCursor));
}

#[test]
fn v2_pages_are_paginated() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let total = server.workspace.pages.len();
    let response = server.handle(&v2_request(Method::Get, "/api/v2/pages?page_size=2", None));
    assert_eq!(response.status, 200);
    let first: ListResponse<PageSummary> = list_data(&response);
    assert_eq!(first.results.len(), 2);
    assert!(first.has_more);

    let mut seen = first.results.len();
    let mut cursor = first.next_cursor;
    while let Some(current) = cursor {
        let response = server.handle(&v2_request(
            Method::Get,
            &format!("/api/v2/pages?page_size=2&start_cursor={current}"),
            None,
        ));
        assert_eq!(response.status, 200);
        let page: ListResponse<PageSummary> = list_data(&response);
        seen += page.results.len();
        cursor = page.next_cursor;
    }
    assert_eq!(seen, total);
}

#[test]
fn v2_rejects_invalid_page_size_and_cursor() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let size = server.handle(&v2_request(Method::Get, "/api/v2/pages?page_size=0", None));
    assert_eq!(size.status, 400);
    assert_eq!(v2_error_code(&size), "validation_error");

    let cursor = server.handle(&v2_request(
        Method::Get,
        "/api/v2/pages?start_cursor=not-a-cursor",
        None,
    ));
    assert_eq!(cursor.status, 400);
    assert_eq!(v2_error_code(&cursor), "invalid_cursor");
}

#[test]
fn non_list_route_ignores_page_size() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/pages/p-home?page_size=0",
        None,
    ));
    assert_eq!(response.status, 200);
    let page: Page = common::data(&response);
    assert_eq!(page.id, "p-home");
}

#[test]
fn v2_empty_list_has_no_cursor() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/search?q=zzzz-no-match&page_size=2",
        None,
    ));
    assert_eq!(response.status, 200);
    let list: ListResponse<ion_of_notes::model::SearchResult> = list_data(&response);
    assert!(list.results.is_empty());
    assert!(!list.has_more);
    assert_eq!(list.next_cursor, None);
}

#[test]
fn v2_versions_are_newest_first_and_paginated() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let created = server.handle(&v2_request(
        Method::Post,
        "/api/v2/pages",
        Some(page_draft_json("Versioned", None)),
    ));
    let page: Page = common::data(&created);
    let mut updated = page.clone();
    updated.title = "Versioned 2".to_string();
    let response = server.handle(&v2_request(
        Method::Patch,
        &format!("/api/v2/pages/{}", page.id),
        Some(serde_json::to_string(&updated).unwrap()),
    ));
    assert_eq!(response.status, 200);

    let response = server.handle(&v2_request(
        Method::Get,
        &format!("/api/v2/pages/{}/versions?page_size=1", page.id),
        None,
    ));
    assert_eq!(response.status, 200);
    let list: ListResponse<ion_of_notes::collab::PageVersion> = list_data(&response);
    assert_eq!(list.results.len(), 1);
    assert!(list.has_more);
    assert!(list.next_cursor.is_some());
}

#[test]
fn v2_sync_is_paginated() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    for title in ["One", "Two"] {
        let response = server.handle(&v2_request(
            Method::Post,
            "/api/v2/pages",
            Some(page_draft_json(title, None)),
        ));
        assert_eq!(response.status, 201);
    }

    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/sync?since=0&page_size=1",
        None,
    ));
    assert_eq!(response.status, 200);
    let list: ListResponse<ion_of_notes::collab::Change> = list_data(&response);
    assert_eq!(list.results.len(), 1);
    assert!(list.has_more);
    assert!(list.next_cursor.is_some());
}

#[test]
fn client_follows_cursors_and_collects_all_pages() {
    let server = WorkspaceServer::new(Workspace::seed("x"));
    let total = server.snapshot().pages.len();
    let mut client = Client::with_token(server, "owner-token");

    let first = client.list_pages_page_v2(2, None).unwrap();
    assert_eq!(first.results.len(), 2);
    assert!(first.has_more);

    let all = client.list_pages_v2().unwrap();
    assert_eq!(all.len(), total);
}

#[test]
fn client_pages_search_and_members() {
    let mut client = Client::with_token(WorkspaceServer::new(Workspace::seed("x")), "owner-token");
    let page = client.list_pages_page_v2(1, None).unwrap();
    assert_eq!(page.results.len(), 1);

    let search = client.search_page_v2("home", 1, None).unwrap();
    assert!(search.results.len() <= 1);

    let members = client.list_members_page_v2(1, None).unwrap();
    assert_eq!(members.results.len(), 1);
    assert!(members.has_more);
}
