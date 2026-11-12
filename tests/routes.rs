//! Route-policy contract tests (spec-078).

use ion_of_notes::server::{BlockLimit, Capability, Method, RoutePolicy, route_policy};

fn policy(method: Method, path: &str) -> &'static RoutePolicy {
    route_policy(method, path).unwrap_or_else(|| panic!("missing policy for {method:?} {path}"))
}

#[test]
fn known_routes_have_one_policy() {
    let workspace = policy(Method::Get, "/api/v2/workspace");
    assert_eq!(workspace.capability, Capability::ReadContent);
    assert!(!workspace.owner_only);
    assert!(!workspace.page_scoped);

    let page = policy(Method::Get, "/api/v2/pages/p-home");
    assert_eq!(page.capability, Capability::ReadContent);
    assert!(page.page_scoped);

    let pages = policy(Method::Get, "/api/v2/pages");
    assert_eq!(pages.capability, Capability::ReadContent);
    assert!(
        !pages.page_scoped,
        "the collection route is not page-scoped"
    );

    let dead_letters = policy(Method::Get, "/api/v2/webhook_deliveries/dead_letters");
    assert_eq!(dead_letters.capability, Capability::ReadContent);

    let delete_webhook = policy(Method::Delete, "/api/v2/webhooks/wh-1");
    assert_eq!(delete_webhook.capability, Capability::UpdateContent);

    let query = policy(Method::Post, "/api/v2/data_sources/ds-1/query");
    assert_eq!(query.capability, Capability::ReadContent);

    let data_source = policy(Method::Post, "/api/v2/databases/db-1/data_sources");
    assert_eq!(data_source.capability, Capability::InsertContent);
}

#[test]
fn unknown_routes_have_no_policy() {
    assert!(route_policy(Method::Get, "/api/v2/not-a-route").is_none());
    assert!(route_policy(Method::Post, "/api/v2/workspace").is_none());
    assert!(route_policy(Method::Delete, "/api/v2/pages").is_none());
}

#[test]
fn owner_only_routes_are_explicit() {
    for (method, path) in [
        (Method::Put, "/api/v2/workspace"),
        (Method::Post, "/api/v2/members"),
        (Method::Put, "/api/v2/shares"),
    ] {
        assert!(
            policy(method, path).owner_only,
            "{method:?} {path} must be owner-only"
        );
    }
    assert!(!policy(Method::Get, "/api/v2/workspace").owner_only);
    assert!(!policy(Method::Post, "/api/v2/pages").owner_only);
}

#[test]
fn block_limit_kinds_are_explicit() {
    assert_eq!(
        policy(Method::Post, "/api/v2/pages").block_limit,
        BlockLimit::PageCreate
    );
    assert_eq!(
        policy(Method::Post, "/api/v2/databases").block_limit,
        BlockLimit::DatabaseCreate
    );
    assert_eq!(
        policy(Method::Patch, "/api/v2/pages/p-home").block_limit,
        BlockLimit::PageReplace
    );
    assert_eq!(
        policy(Method::Patch, "/api/v2/blocks/b-1/children").block_limit,
        BlockLimit::BlockChildren
    );
    assert_eq!(
        policy(Method::Get, "/api/v2/workspace").block_limit,
        BlockLimit::None
    );
}

#[test]
fn page_scoped_routes_are_explicit() {
    assert!(policy(Method::Get, "/api/v2/pages/p-home").page_scoped);
    assert!(policy(Method::Patch, "/api/v2/pages/p-home").page_scoped);
    assert!(policy(Method::Delete, "/api/v2/pages/p-home").page_scoped);
    assert!(policy(Method::Get, "/api/v2/pages/p-home/versions").page_scoped);
    assert!(!policy(Method::Post, "/api/v2/pages").page_scoped);
    assert!(!policy(Method::Get, "/api/v2/blocks/b-1").page_scoped);
}
