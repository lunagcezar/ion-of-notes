//! Opt-in contention harness for the single workspace lock (spec-126).
//!
//! Run with:
//! `ION_OF_NOTES_LOAD=1 cargo test --no-default-features --test load -- --nocapture`
//!
//! The default test is a no-op so the normal gate stays deterministic and fast.
//! The harness measures read and write latency while several threads contend
//! for the same `WorkspaceServer`; it is a measurement tool, not a hard
//! latency gate.

use ion_of_notes::model::{Block, BlockKind, Page, Workspace};
use ion_of_notes::server::{ApiRequest, Method, RequestTransport, WorkspaceServer};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const PAGES: usize = 1_000;
const BLOCKS_PER_PAGE: usize = 10;
const THREADS: usize = 8;
const ITERATIONS: usize = 100;

#[test]
fn concurrent_reads_and_writes_measure_the_workspace_lock() {
    if std::env::var_os("ION_OF_NOTES_LOAD").is_none() {
        return;
    }

    let mut workspace = Workspace::new("load");
    for page_index in 0..PAGES {
        let id = format!("load-{page_index}");
        let mut page = Page::new(id, format!("Load {page_index}"), "");
        page.blocks = (0..BLOCKS_PER_PAGE)
            .map(|block_index| {
                Block::new(
                    format!("load-{page_index}-b{block_index}"),
                    BlockKind::Paragraph,
                    "A paragraph used by the contention harness.",
                )
            })
            .collect();
        workspace.add_page(page).expect("unique load page");
    }

    let server = Arc::new(Mutex::new(WorkspaceServer::new(workspace)));
    let edit_body = Arc::new(
        serde_json::to_string(
            &server
                .lock()
                .expect("lock")
                .workspace
                .page("load-0")
                .cloned(),
        )
        .unwrap_or_default(),
    );
    let durations = Arc::new(Mutex::new(Vec::with_capacity(THREADS * ITERATIONS)));

    let mut handles = Vec::new();
    for thread_index in 0..THREADS {
        let server = Arc::clone(&server);
        let durations = Arc::clone(&durations);
        let edit_body = Arc::clone(&edit_body);
        handles.push(thread::spawn(move || {
            for iteration in 0..ITERATIONS {
                let request = if (thread_index + iteration) % 2 == 0 {
                    ApiRequest::new(Method::Get, "/api/pages", None)
                } else {
                    ApiRequest::new(
                        Method::Patch,
                        "/api/pages/load-0",
                        Some(edit_body.as_ref().clone()),
                    )
                };
                let started = Instant::now();
                let response = server.lock().expect("lock").handle(&request);
                let elapsed = started.elapsed();
                assert!(
                    (200..300).contains(&response.status),
                    "{} {} returned {}",
                    request.method.as_str(),
                    request.path,
                    response.status
                );
                durations.lock().expect("durations").push(elapsed);
            }
        }));
    }
    for handle in handles {
        handle.join().expect("load thread");
    }

    let mut samples = durations.lock().expect("durations").clone();
    samples.sort_unstable();
    let percentile = |percent: usize| {
        let index = (samples.len().saturating_sub(1) * percent) / 100;
        samples[index]
    };
    println!(
        "load: pages={PAGES} blocks={} threads={THREADS} iterations={ITERATIONS} p50={:?} p95={:?} max={:?}",
        PAGES * BLOCKS_PER_PAGE,
        percentile(50),
        percentile(95),
        samples.last().copied().unwrap_or(Duration::ZERO)
    );
}
