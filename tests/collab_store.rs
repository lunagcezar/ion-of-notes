//! Tests for collaboration persistence (spec-019).

mod common;

use common::{page_draft_json, remove, temp_path};
use ion_of_notes::collab::{Access, CollabState, MemberDraft, Role, Share};
use ion_of_notes::model::Workspace;
use ion_of_notes::server::{ApiRequest, Method, WorkspaceApi, WorkspaceServer};
use ion_of_notes::store::{
    CollabStore, JsonCollabStore, NoCollabStore, NoStore, load_collab, load_collab_or_seed,
    save_collab,
};

fn member_draft(name: &str, email: &str) -> String {
    serde_json::to_string(&MemberDraft {
        name: name.to_string(),
        email: email.to_string(),
        role: Role::Guest,
    })
    .unwrap()
}

#[test]
fn json_collab_store_roundtrip() {
    let path = temp_path("collab-roundtrip", "json");
    remove(&path);
    let mut state = CollabState::new();
    state.invite(MemberDraft {
        name: "Round Trip".to_string(),
        email: "round@workspace.local".to_string(),
        role: Role::Guest,
    });
    state.set_access(Share {
        page_id: "p1".to_string(),
        subject: "m-guest-1".to_string(),
        access: Access::View,
    });

    let store = JsonCollabStore::new(path.clone());
    store.persist(&state).unwrap();
    assert_eq!(load_collab(&path).unwrap(), state);

    save_collab(&path, &state).unwrap();
    assert_eq!(load_collab(&path).unwrap(), state);
    remove(&path);
}

#[test]
fn missing_collab_file_is_seeded_and_saved() {
    let path = temp_path("collab-seed", "json");
    remove(&path);
    let state = load_collab_or_seed(&path);
    assert!(!state.members.is_empty());
    assert!(path.exists());
    assert_eq!(load_collab(&path).unwrap(), state);
    remove(&path);
}

#[test]
fn corrupt_collab_file_falls_back_to_seed() {
    let path = temp_path("collab-corrupt", "json");
    std::fs::write(&path, "not json at all").unwrap();
    let state = load_collab_or_seed(&path);
    assert!(!state.members.is_empty());
    remove(&path);
}

#[test]
fn server_restart_restores_members_shares_changes_and_versions() {
    let workspace_path = temp_path("collab-ws", "json");
    let collab_path = temp_path("collab-state", "json");
    remove(&workspace_path);
    remove(&collab_path);
    let workspace = Workspace::seed("persist");

    {
        let mut server = WorkspaceServer::with_paths(
            workspace.clone(),
            workspace_path.clone(),
            collab_path.clone(),
        );

        let invited = server.handle(&ApiRequest::new(
            Method::Post,
            "/api/members",
            Some(member_draft("Persisted Guest", "persisted@workspace.local")),
        ));
        assert_eq!(invited.status, 201);

        let share = Share {
            page_id: "p-home".to_string(),
            subject: "m-guest-1".to_string(),
            access: Access::View,
        };
        let shared = server.handle(&ApiRequest::new(
            Method::Put,
            "/api/shares",
            Some(serde_json::to_string(&share).unwrap()),
        ));
        assert_eq!(shared.status, 200);

        let created = server.handle(&ApiRequest::new(
            Method::Post,
            "/api/pages",
            Some(page_draft_json("Persisted page", None)),
        ));
        assert_eq!(created.status, 201);
    }

    let restarted =
        WorkspaceServer::with_paths(workspace, workspace_path.clone(), collab_path.clone());
    assert_eq!(restarted.collab().members.len(), 4);
    assert!(
        restarted
            .collab()
            .members
            .iter()
            .any(|m| m.name == "Persisted Guest")
    );
    assert!(
        restarted
            .collab()
            .shares
            .iter()
            .any(|s| s.page_id == "p-home" && s.access == Access::View)
    );
    assert!(!restarted.collab().changes.is_empty());
    assert!(!restarted.collab().versions.is_empty());

    remove(&workspace_path);
    remove(&collab_path);
}

#[derive(Clone, Default)]
struct FailingCollabStore;

impl CollabStore for FailingCollabStore {
    fn persist(&self, _state: &CollabState) -> std::io::Result<()> {
        Err(std::io::Error::other("simulated collab failure"))
    }
}

#[test]
fn collaboration_persistence_failure_is_reported_as_500() {
    let mut server = WorkspaceServer::with_stores(
        Workspace::new("x"),
        CollabState::new(),
        NoStore,
        FailingCollabStore,
    );

    let invited = server.handle(&ApiRequest::new(
        Method::Post,
        "/api/members",
        Some(member_draft("No Persist", "nopersist@workspace.local")),
    ));
    assert_eq!(invited.status, 500);
    assert!(invited.body.contains("simulated collab failure"));

    let shared = server.handle(&ApiRequest::new(
        Method::Put,
        "/api/shares",
        Some(
            serde_json::to_string(&Share {
                page_id: "p1".to_string(),
                subject: "m-guest-1".to_string(),
                access: Access::Full,
            })
            .unwrap(),
        ),
    ));
    assert_eq!(shared.status, 500);
    assert!(shared.body.contains("simulated collab failure"));
}

#[test]
fn no_collab_store_is_a_noop() {
    assert!(NoCollabStore.persist(&CollabState::new()).is_ok());
}
