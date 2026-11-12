//! Page properties, data sources and property-item tests (spec-033).

mod common;

use common::{data, list_data, v2_request};
use ion_of_notes::client::Client;
use ion_of_notes::model::{
    DataSource, DataSourceInput, Database, DatabaseInput, DateValue, OptionRef, Page,
    PagePropertiesInput, PageProperty, Property, PropertyConfig, PropertyType, PropertyValue,
    RichText, SelectOption, StatusGroup, Workspace,
};
use ion_of_notes::server::{ListResponse, Method, RequestTransport, WorkspaceServer};
use std::collections::BTreeMap;

fn status_property() -> Property {
    let options = vec![
        SelectOption::new("opt-not-started", "Not started", "gray"),
        SelectOption::new("opt-in-progress", "In progress", "blue"),
        SelectOption::new("opt-done", "Done", "green"),
    ];
    Property::new(
        "status",
        "Status",
        PropertyType::Status,
        PropertyConfig::Status {
            default_option: Some("Not started".to_string()),
            options,
            groups: vec![StatusGroup {
                id: "group-todo".to_string(),
                name: "To-do".to_string(),
                color: "gray".to_string(),
                option_ids: vec!["opt-not-started".to_string()],
            }],
        },
    )
}

#[test]
fn property_schema_roundtrips() {
    let property = status_property();
    let json = serde_json::to_string(&property).unwrap();
    let back: Property = serde_json::from_str(&json).unwrap();
    assert_eq!(back, property);
    assert_eq!(back.r#type, PropertyType::Status);

    let date = Property::new("due", "Due", PropertyType::Date, PropertyConfig::None);
    assert_eq!(
        serde_json::from_str::<Property>(&serde_json::to_string(&date).unwrap()).unwrap(),
        date
    );
}

#[test]
fn page_property_values_roundtrip() {
    let values = vec![
        PageProperty::new(
            "title",
            PropertyType::Title,
            PropertyValue::Title(vec![RichText::text("Task")]),
        ),
        PageProperty::new(
            "number",
            PropertyType::Number,
            PropertyValue::Number(Some(8)),
        ),
        PageProperty::new(
            "select",
            PropertyType::Select,
            PropertyValue::Select(Some(OptionRef {
                id: "opt-done".to_string(),
                name: "Done".to_string(),
                color: "green".to_string(),
            })),
        ),
        PageProperty::new(
            "date",
            PropertyType::Date,
            PropertyValue::Date(Some(DateValue::new("2026-11-08"))),
        ),
        PageProperty::new(
            "checkbox",
            PropertyType::Checkbox,
            PropertyValue::Checkbox(true),
        ),
    ];
    for value in values {
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<PageProperty>(&json).unwrap(), value);
    }
}

#[test]
fn clear_rules_match_property_types() {
    assert_eq!(
        PropertyValue::clear(PropertyType::Title),
        PropertyValue::Title(Vec::new())
    );
    assert_eq!(
        PropertyValue::clear(PropertyType::Number),
        PropertyValue::Number(None)
    );
    assert_eq!(
        PropertyValue::clear(PropertyType::Checkbox),
        PropertyValue::Checkbox(false)
    );
    assert_eq!(
        PropertyValue::clear(PropertyType::MultiSelect),
        PropertyValue::MultiSelect(Vec::new())
    );
    assert_eq!(
        PropertyValue::clear(PropertyType::Relation),
        PropertyValue::Relation(Vec::new())
    );
    assert_eq!(
        PropertyValue::clear(PropertyType::Status),
        PropertyValue::Status(None)
    );
}

#[test]
fn default_data_source_migrates_table_rows() {
    let mut workspace = Workspace::seed("x");
    workspace.ensure_data_sources();

    let database = workspace.database("db-p-tasks").expect("tasks database");
    assert_eq!(database.data_sources.len(), 1);
    let data_source = workspace
        .data_source("ds-p-tasks")
        .expect("tasks data source");
    assert!(data_source.properties.contains_key("title"));
    assert!(data_source.properties.contains_key("status"));
    assert!(data_source.properties.contains_key("assignee"));
    assert!(data_source.properties.contains_key("due"));
    assert_eq!(data_source.rows.len(), 5);

    let row = &data_source.rows[0];
    assert_eq!(row.data_source_id.as_deref(), Some("ds-p-tasks"));
    assert_eq!(row.properties.len(), 4);
    assert_eq!(
        row.properties.get("title").map(|p| p.r#type),
        Some(PropertyType::Title)
    );

    let count = workspace.data_sources.len();
    workspace.ensure_data_sources();
    assert_eq!(workspace.data_sources.len(), count);
}

#[test]
fn v2_lists_databases_and_data_sources() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(Method::Get, "/api/v2/databases", None));
    assert_eq!(response.status, 200);
    let databases: Vec<Database> = data(&response);
    assert!(databases.iter().any(|db| db.id == "db-p-tasks"));

    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/databases/db-p-tasks/data_sources",
        None,
    ));
    assert_eq!(response.status, 200);
    let sources: Vec<DataSource> = data(&response);
    assert!(sources.iter().any(|source| source.id == "ds-p-tasks"));
}

#[test]
fn v2_creates_database_and_data_source() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let database_input = DatabaseInput {
        title: "Projects".to_string(),
        parent_page_id: Some("p-home".to_string()),
        is_inline: true,
    };
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/databases",
        Some(serde_json::to_string(&database_input).unwrap()),
    ));
    assert_eq!(response.status, 201);
    let database: Database = data(&response);
    assert!(!database.id.is_empty());

    let source_input = DataSourceInput {
        title: "Tasks".to_string(),
        properties: BTreeMap::from([
            (
                "title".to_string(),
                Property::new("title", "Name", PropertyType::Title, PropertyConfig::None),
            ),
            ("status".to_string(), status_property()),
        ]),
    };
    let response = server.handle(&v2_request(
        Method::Post,
        &format!("/api/v2/databases/{}/data_sources", database.id),
        Some(serde_json::to_string(&source_input).unwrap()),
    ));
    assert_eq!(response.status, 201);
    let source: DataSource = data(&response);
    assert_eq!(source.properties.len(), 2);

    let response = server.handle(&v2_request(
        Method::Get,
        &format!("/api/v2/databases/{}", database.id),
        None,
    ));
    let refreshed: Database = data(&response);
    assert!(
        refreshed
            .data_sources
            .iter()
            .any(|ref_| ref_.id == source.id)
    );
}

#[test]
fn v2_data_source_query_paginates_rows() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));
    let response = server.handle(&v2_request(
        Method::Post,
        "/api/v2/data_sources/ds-p-tasks/query?page_size=2",
        None,
    ));
    assert_eq!(response.status, 200);
    let first: ListResponse<Page> = list_data(&response);
    assert_eq!(first.results.len(), 2);
    assert!(first.has_more);

    let response = server.handle(&v2_request(
        Method::Post,
        &format!(
            "/api/v2/data_sources/ds-p-tasks/query?page_size=2&start_cursor={}",
            first.next_cursor.unwrap()
        ),
        None,
    ));
    let second: ListResponse<Page> = list_data(&response);
    assert!(second.results.len() <= 3);
}

#[test]
fn v2_page_property_items_and_write_clear() {
    let mut server = WorkspaceServer::new(Workspace::seed("x"));

    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/pages/row-r1/properties/status",
        None,
    ));
    assert_eq!(response.status, 200);
    let items: ListResponse<PageProperty> = list_data(&response);
    assert_eq!(items.results.len(), 1);
    assert_eq!(items.results[0].r#type, PropertyType::Status);

    let update = PagePropertiesInput {
        properties: BTreeMap::from([(
            "status".to_string(),
            PageProperty::new(
                "status",
                PropertyType::Status,
                PropertyValue::Status(Some(OptionRef {
                    id: "opt-done".to_string(),
                    name: "Done".to_string(),
                    color: "green".to_string(),
                })),
            ),
        )]),
    };
    let response = server.handle(&v2_request(
        Method::Patch,
        "/api/v2/pages/row-r1/properties",
        Some(serde_json::to_string(&update).unwrap()),
    ));
    assert_eq!(response.status, 200);

    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/pages/row-r1/properties/status",
        None,
    ));
    let items: ListResponse<PageProperty> = list_data(&response);
    match &items.results[0].value {
        PropertyValue::Status(Some(option)) => assert_eq!(option.name, "Done"),
        other => panic!("expected Done status, got {other:?}"),
    }

    let clear = PagePropertiesInput {
        properties: BTreeMap::from([(
            "status".to_string(),
            PageProperty::new(
                "status",
                PropertyType::Status,
                PropertyValue::clear(PropertyType::Status),
            ),
        )]),
    };
    let response = server.handle(&v2_request(
        Method::Patch,
        "/api/v2/pages/row-r1/properties",
        Some(serde_json::to_string(&clear).unwrap()),
    ));
    assert_eq!(response.status, 200);
    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/pages/row-r1/properties/status",
        None,
    ));
    let items: ListResponse<PageProperty> = list_data(&response);
    assert_eq!(items.results[0].value, PropertyValue::Status(None));

    // Omitted properties are preserved.
    let response = server.handle(&v2_request(
        Method::Get,
        "/api/v2/pages/row-r1/properties/title",
        None,
    ));
    let items: ListResponse<PageProperty> = list_data(&response);
    assert_eq!(items.results.len(), 1);
}

#[test]
fn client_property_roundtrip() {
    let mut client = Client::with_token(WorkspaceServer::new(Workspace::seed("x")), "owner-token");
    let databases = client.list_databases_v2().unwrap();
    assert!(databases.iter().any(|db| db.id == "db-p-tasks"));

    let sources = client.list_data_sources_v2("db-p-tasks").unwrap();
    assert!(sources.iter().any(|source| source.id == "ds-p-tasks"));

    let rows = client.query_data_source_v2("ds-p-tasks").unwrap();
    assert_eq!(rows.len(), 5);

    let items = client.page_property_items_v2("row-r1", "title").unwrap();
    assert_eq!(items.len(), 1);
}
