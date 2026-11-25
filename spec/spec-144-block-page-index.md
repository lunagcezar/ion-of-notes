# spec-144 - Block-to-page index

- Status: implemented (2026-11-25)
- Related: `spec-022`, `spec-133`, `docs/performance-roadmap.md`
- Review source: 2026-11-25 ChatGPT latency review, item 3.4

## Purpose and scope

`Workspace::block`, `block_mut` and `page_id_for_block` scanned every page and
every nested block. The editor resolves blocks constantly, so this was linear in
the whole workspace for each edit.

### Index

`WorkspaceIndex` now also keeps `block_id -> page_id`, built with the page
index and rebuilt when pages are added/removed or the workspace is
deserialized. `Workspace::blocks_changed()` rebuilds only the block map after a
structural change that bypasses the page-tree methods (UI split/insert/delete/
drop and server block appends, plus CRDT structure projection).

### Safe fallback

A hit is verified against the actual page tree. If a block was moved without a
refresh, the stale entry fails verification and lookup falls back to the scan,
so the optimization can never return the wrong page. A miss also falls back.

## Acceptance criteria

P1. **Given** a seeded workspace, **then** `block("h2")` and
`page_id_for_block("h2")` resolve to `p-home` through the index.

P2. **Given** a block inserted directly into a page plus `blocks_changed()`,
**then** lookup resolves to that page.

P3. **Given** a removed page, **then** its blocks are no longer found.

P4. **Given** a stale index, **then** lookup falls back to the tree and returns
the real page.

## Test scenarios

- `tests/model.rs`
  (`block_lookup_uses_the_derived_index_and_survives_structural_changes`).
- Existing block, CRDT and UI suites.

## Plan / tasks

- [x] Add the block map to `WorkspaceIndex`.
- [x] Use it in `block`, `block_mut` and `page_id_for_block` with verification.
- [x] Refresh on page changes, structural block commands, server appends and
      CRDT projection.
- [x] Add the regression test.
