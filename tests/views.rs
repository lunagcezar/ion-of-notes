//! View and database-layout tests (spec-034).

mod common;

use common::{data, v2_request};
use ion_of_notes::client::Client;
use ion_of_notes::model::{
    Filter, Page, Sort, SortDirection, View, ViewConfig, ViewInput, ViewQueryInput,
    ViewQueryResult, ViewType, Workspace,
};
use ion_of_notes::server::{Method, RequestTransport, WorkspaceServer};

#[test]
fn view_model_roundtrips() {
    let view = View {
        id: "view-1".to_string(),
        parent: ion_of_notes::model::ParentDto::Database {
            database_id: "db-1".to_string(),
        },
        data_source_id: Some("ds-1".to_string()),
        name: "Board".to_string(),
        r#type: ViewType::Board,
        filter: Some(Filter::Equals {
            property: "status".to_string(),
            value: "Done".to_string(),
        }),
        sorts: Some(vec![Sort::new("due", SortDirection::Ascending)]),
        configuration: Some(ViewConfig::Board {
            group_by: "status".to_string(),
        }),
        created_time: "2026-11-08T00:00:00Z".to_string(),
        created_by: None,
        last_edited_time: "2026-11-08T00:00:00Z".to_string(),
        last_edited_by: None,
        url: String::new(),
    };
    let json = serde_json::to_string(&view).unwrap();
    assert_eq!(serde_json::from_str::<View>(&json).unwrap(), view);
}

#[test]
fn default_views_are_created() {
    let mut workspace = Workspace::seed("x");
    workspace.ensure_data_sources();
    workspace.ensure_views();
    let view = workspace.view("view-ds-p-tasks").expect("default view");
    assert_eq!(view.r#type, ViewType::Table);
    assert_eq!(view.data_source_id.as_deref(), Some("ds-p-tasks"));
}

#[test]
fn v2_lists_views() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(Method::Get, "/api/v2/views", None));
    assert_eq!(response.status, 200);
    let views: Vec<View> = data(&response);
    assert!(views.iter().any(|view| view.id == "view-ds-p-tasks"));

    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/views?data_source=ds-p-tasks",
        None,
    ));
    let views: Vec<View> = data(&response);
    assert!(
        views
            .iter()
            .all(|view| view.data_source_id.as_deref() == Some("ds-p-tasks"))
    );
}

#[test]
fn v2_creates_updates_and_deletes_view() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let input = ViewInput {
        name: "Status board".to_string(),
        data_source_id: Some("ds-p-tasks".to_string()),
        r#type: ViewType::Board,
        filter: Some(Filter::Equals {
            property: "status".to_string(),
            value: "Done".to_string(),
        }),
        sorts: Some(vec![Sort::new("due", SortDirection::Descending)]),
        configuration: Some(ViewConfig::Board {
            group_by: "status".to_string(),
        }),
    };
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/views",
        Some(serde_json::to_string(&input).unwrap()),
    ));
    assert_eq!(response.status, 201);
    let view: View = data(&response);
    assert_eq!(view.r#type, ViewType::Board);

    let update = ViewInput {
        name: "Renamed board".to_string(),
        ..input
    };
    let response = server.handle(&v2_request(
        Method::Patch,
        &format!("/api/v2/views/{}", view.id),
        Some(serde_json::to_string(&update).unwrap()),
    ));
    assert_eq!(response.status, 200);
    let updated: View = data(&response);
    assert_eq!(updated.name, "Renamed board");

    let response = server.handle(&v2_request(
        Method::Delete,
        &format!("/api/v2/views/{}", view.id),
        None,
    ));
    assert_eq!(response.status, 200);

    let response = server.handle(&v2_request(
        Method::Get,
        &format!("/api/v2/views/{}", view.id),
        None,
    ));
    assert_eq!(response.status, 404);
}

#[test]
fn v2_table_query_filters_and_sorts() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let query = ViewQueryInput {
        filter: Some(Filter::Equals {
            property: "status".to_string(),
            value: "Done".to_string(),
        }),
        sorts: Some(vec![Sort::new("due", SortDirection::Ascending)]),
    };
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/views/view-ds-p-tasks/query",
        Some(serde_json::to_string(&query).unwrap()),
    ));
    assert_eq!(response.status, 200);
    let result: ViewQueryResult = data(&response);
    assert!(!result.pages.is_empty());
    assert!(result.pages.iter().all(|page| {
        page.properties
            .get("status")
            .and_then(|property| match &property.value {
                ion_of_notes::model::PropertyValue::Status(Some(option)) => {
                    Some(option.name.as_str())
                }
                _ => None,
            })
            == Some("Done")
    }));
}

#[test]
fn v2_board_query_groups_by_status() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let input = ViewInput {
        name: "Board".to_string(),
        data_source_id: Some("ds-p-tasks".to_string()),
        r#type: ViewType::Board,
        configuration: Some(ViewConfig::Board {
            group_by: "status".to_string(),
        }),
        ..ViewInput::default()
    };
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/views",
        Some(serde_json::to_string(&input).unwrap()),
    ));
    let view: View = data(&response);

    let response = server.handle(&v2_request(
        Method::Post,
        &format!("/api/v2/views/{}/query", view.id),
        None,
    ));
    assert_eq!(response.status, 200);
    let result: ViewQueryResult = data(&response);
    assert!(result.groups.iter().any(|group| group.name == "Done"));
    assert!(
        result
            .groups
            .iter()
            .any(|group| group.name == "In progress")
    );
}

#[test]
fn v2_calendar_query_groups_by_date() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let input = ViewInput {
        name: "Calendar".to_string(),
        data_source_id: Some("ds-p-tasks".to_string()),
        r#type: ViewType::Calendar,
        configuration: Some(ViewConfig::Calendar {
            date_property: "due".to_string(),
        }),
        ..ViewInput::default()
    };
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/views",
        Some(serde_json::to_string(&input).unwrap()),
    ));
    let view: View = data(&response);
    let response = server.handle(&v2_request(
        Method::Post,
        &format!("/api/v2/views/{}/query", view.id),
        None,
    ));
    let result: ViewQueryResult = data(&response);
    assert!(!result.groups.is_empty());
    assert!(result.groups.iter().all(|group| !group.pages.is_empty()));
}

#[test]
fn v2_view_query_paginates() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/views/view-ds-p-tasks/query?page_size=2",
        None,
    ));
    assert_eq!(response.status, 200);
    let result: ViewQueryResult = data(&response);
    assert_eq!(result.pages.len(), 2);
    assert!(result.has_more);
    assert!(result.next_cursor.is_some());

    let response = server.handle(&v2_request(
        Method::Post,
        &format!(
            "/api/v2/views/view-ds-p-tasks/query?page_size=2&start_cursor={}",
            result.next_cursor.unwrap()
        ),
        None,
    ));
    let second: ViewQueryResult = data(&response);
    assert!(second.pages.len() <= 3);
}

#[test]
fn client_view_roundtrip() {
    let mut client = Client::with_token(WorkspaceServer::new(Workspace::seed("x")), "owner-token");
    let views = client.list_views_v2(Some("ds-p-tasks")).unwrap();
    assert!(views.iter().any(|view| view.id == "view-ds-p-tasks"));

    let input = ViewInput {
        name: "Client board".to_string(),
        data_source_id: Some("ds-p-tasks".to_string()),
        r#type: ViewType::Board,
        configuration: Some(ViewConfig::Board {
            group_by: "status".to_string(),
        }),
        ..ViewInput::default()
    };
    let view = client.create_view_v2(&input).unwrap();
    let result = client.query_view_v2(&view.id, None, 100, None).unwrap();
    assert!(!result.groups.is_empty());
}

#[test]
fn query_filter_and_sort_are_typed() {
    let filter = Filter::And {
        filters: vec![
            Filter::Equals {
                property: "status".to_string(),
                value: "Done".to_string(),
            },
            Filter::IsNotEmpty {
                property: "due".to_string(),
            },
        ],
    };
    let json = serde_json::to_string(&filter).unwrap();
    assert_eq!(serde_json::from_str::<Filter>(&json).unwrap(), filter);

    let page = Page::new("p1", "One", "1");
    let _ = page;
}
