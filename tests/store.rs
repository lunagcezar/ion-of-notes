//! Tests for JSON persistence (public API only).

mod common;

use common::{remove, temp_path};
use ion_of_notes::model::{Workspace, WorkspaceCatalog};
use ion_of_notes::store;

#[test]
fn workspace_catalog_roundtrip_keeps_the_active_workspace() {
    let workspace_path = temp_path("catalog-workspace", "json");
    let catalog_path = temp_path("catalog", "json");
    let mut first = Workspace::seed("First");
    first.settings.id = "first".to_string();
    let mut second = Workspace::seed("Second");
    second.settings.id = "second".to_string();
    let mut catalog = WorkspaceCatalog::new(first);
    catalog.add(second);
    store::save_catalog(&catalog_path, &catalog).unwrap();
    let loaded = store::load_catalog_or_seed(&catalog_path, &workspace_path).unwrap();
    assert_eq!(loaded.summaries().len(), 2);
    assert_eq!(loaded.active_id, "second");
    remove(&catalog_path);
    remove(&workspace_path);
}

#[test]
fn save_and_load_roundtrip() {
    let path = temp_path("roundtrip", "json");
    let ws = Workspace::seed("disk");
    store::save(&path, &ws).unwrap();
    let loaded = store::load(&path).unwrap();
    assert_eq!(ws, loaded);
    remove(&path);
}

#[test]
fn save_is_atomic_and_leaves_no_temporary_file() {
    let path = temp_path("atomic", "json");
    remove(&path);
    let ws = Workspace::seed("atomic");
    store::save(&path, &ws).unwrap();
    assert_eq!(store::load(&path).unwrap(), ws);
    let temporary = path.with_extension("tmp");
    assert!(!temporary.exists(), "temporary save file must be removed");
    remove(&path);
}

#[test]
fn load_or_seed_creates_file() {
    let path = temp_path("seed", "json");
    remove(&path);
    let ws = store::load_or_seed(&path).unwrap();
    assert!(path.exists());
    assert!(!ws.pages.is_empty());
    remove(&path);
}

#[test]
fn corrupt_file_is_preserved_and_loading_fails() {
    let path = temp_path("corrupt", "json");
    std::fs::write(&path, "not json at all").unwrap();
    let error = store::load_or_seed(&path).expect_err("corrupt data must not be replaced");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "not json at all",
        "the original file is the recovery evidence"
    );
    remove(&path);
}
