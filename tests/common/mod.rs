//! Shared helpers for the integration-test crates.
//!
//! Every file in `tests/` is compiled as its own crate, so shared helpers live
//! in a `common/` submodule rather than `tests/common.rs` — that keeps Cargo
//! from reporting an extra empty test target (Rust Book, ch. 11). Keeping the
//! helpers in one place is the DRY rule: the old inline test modules each had
//! their own copy of the temporary-file generator.

#![allow(dead_code)]

use not_notion::model::PageDraft;
use not_notion::server::{
    API_VERSION, ApiRequest, ApiResponse, Envelope, ListResponse, Method, OWNER_TOKEN,
};
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

/// Build a v2 request as the local owner.
pub fn v2_request(method: Method, path: &str, body: Option<String>) -> ApiRequest {
    v2_request_as(OWNER_TOKEN, method, path, body)
}

/// Build a v2 request as a specific connection token.
pub fn v2_request_as(token: &str, method: Method, path: &str, body: Option<String>) -> ApiRequest {
    ApiRequest::new(method, path, body)
        .with_header("not-notion-version", API_VERSION)
        .with_header("authorization", format!("Bearer {token}"))
}

/// Decode a v2 list envelope.
pub fn list_data<T: DeserializeOwned>(response: &ApiResponse) -> ListResponse<T> {
    let envelope: Envelope<ListResponse<T>> = serde_json::from_str(&response.body).unwrap();
    envelope.data.expect("list envelope has no data")
}
