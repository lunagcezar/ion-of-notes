//! Shared helpers for the integration-test crates.
//!
//! Every file in `tests/` is compiled as its own crate, so shared helpers live
//! in a `common/` submodule rather than `tests/common.rs` — that keeps Cargo
//! from reporting an extra empty test target (Rust Book, ch. 11). Keeping the
//! helpers in one place is the DRY rule: the old inline test modules each had
//! their own copy of the temporary-file generator.

#![allow(dead_code)]

use not_notion::model::PageDraft;
use not_notion::server::{ApiResponse, Envelope};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique temporary path for one test artifact.
pub fn temp_path(tag: &str, extension: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "not-notion-{tag}-{}-{n}.{extension}",
        std::process::id()
    ))
}

/// Best-effort removal of a test artifact (missing files are fine).
pub fn remove(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// JSON body for a create-page request.
pub fn page_draft_json(title: &str, parent: Option<&str>) -> String {
    serde_json::to_string(&PageDraft {
        title: title.to_string(),
        icon: "P".to_string(),
        parent: parent.map(str::to_string),
    })
    .unwrap()
}

/// Decode the `data` field of a successful JSON envelope.
pub fn data<T: DeserializeOwned>(response: &ApiResponse) -> T {
    let envelope: Envelope<T> = serde_json::from_str(&response.body).unwrap();
    envelope.data.expect("response envelope has no data")
}
