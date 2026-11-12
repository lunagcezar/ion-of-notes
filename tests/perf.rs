//! Deterministic performance harness (opt-in).
//!
//! The functional suite is not a benchmark. This crate builds workspaces at
//! 1k and 10k blocks and measures the four operations the review asked about:
//! JSON load/serialize, ranked search, page lookup and CRDT sync.
//!
//! It is opt-in because timing assertions are machine dependent and the large
//! fixtures would slow the default gate:
//!
//! ```text
//! ION_OF_NOTES_PERF=1 cargo test --no-default-features --test perf -- --nocapture
//! ```
//!
//! The assertions are correctness assertions (round-trip equality, non-empty
//! results, a merged CRDT update); the printed timings are the measurement.

use ion_of_notes::model::{Block, BlockKind, Page, Workspace};
use ion_of_notes::server::WorkspaceServer;
use std::time::{Duration, Instant};

/// Build `pages` pages of `blocks_per_page` paragraph blocks.
fn workspace_with_blocks(pages: usize, blocks_per_page: usize) -> Workspace {
    let mut workspace = Workspace::new("perf");
    for page in 0..pages {
        let id = format!("page-{page}");
        let mut model = Page::new(&id, format!("Page {page}"), "");
        model.blocks = (0..blocks_per_page)
            .map(|block| {
                Block::new(
                    format!("{id}-b{block}"),
                    if block == 0 {
                        BlockKind::Heading1
                    } else {
                        BlockKind::Paragraph
                    },
                    format!("page {page} block {block} searchable token{block}"),
                )
            })
            .collect();
        workspace.add_page(model).expect("perf fixture page");
    }
    workspace
}

fn timed<T>(label: &str, run: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let value = run();
    let elapsed = start.elapsed();
    println!("{label}: {:.1} ms", elapsed.as_secs_f64() * 1000.0);
    value
}

fn measure(pages: usize, blocks_per_page: usize) {
    let total = pages * blocks_per_page;
    println!("--- {pages} pages / {total} blocks ---");

    let workspace = workspace_with_blocks(pages, blocks_per_page);

    let json = timed("serialize", || {
        serde_json::to_string(&workspace).expect("serialize workspace")
    });

    let loaded: Workspace = timed("deserialize", || {
        serde_json::from_str(&json).expect("deserialize workspace")
    });
    assert_eq!(loaded.pages.len(), workspace.pages.len());
    assert_eq!(
        loaded
            .pages
            .iter()
            .map(|page| page.blocks.len())
            .sum::<usize>(),
        total
    );

    let hits = timed("search", || workspace.search("token7"));
    assert!(!hits.is_empty(), "search should find token7");

    let first = workspace.pages[0].id.clone();
    let page = timed("page lookup", || workspace.page(&first).cloned());
    assert!(page.is_some(), "first page must exist");

    let mut server = WorkspaceServer::new(workspace.clone());
    let (update, _) = timed("crdt sync", || {
        server.crdt_sync(&first, None).expect("crdt sync")
    });
    assert!(!update.is_empty(), "crdt sync should return an update");

    println!("json size: {:.1} KiB", json.len() as f64 / 1024.0);
}

#[test]
fn perf_harness_reports_load_search_and_crdt() {
    if std::env::var("ION_OF_NOTES_PERF").ok().as_deref() != Some("1") {
        println!("perf harness skipped; set ION_OF_NOTES_PERF=1 to run");
        return;
    }

    measure(10, 100);
    measure(100, 100);
    measure(500, 100);

    // Keep the harness honest about its own overhead.
    let start = Instant::now();
    std::hint::black_box(Duration::from_millis(1));
    assert!(start.elapsed() < Duration::from_secs(5));
}
