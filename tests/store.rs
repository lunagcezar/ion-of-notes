//! Tests for JSON persistence (public API only).

mod common;

use common::{remove, temp_path};
use not_notion::model::Workspace;
use not_notion::store;

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
    let ws = store::load_or_seed(&path);
    assert!(path.exists());
    assert!(!ws.pages.is_empty());
    remove(&path);
}

#[test]
fn corrupt_file_falls_back_to_seed() {
    let path = temp_path("corrupt", "json");
    std::fs::write(&path, "not json at all").unwrap();
    let ws = store::load_or_seed(&path);
    assert!(!ws.pages.is_empty());
    remove(&path);
}
