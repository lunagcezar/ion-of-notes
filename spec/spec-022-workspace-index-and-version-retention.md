# spec-022 — Workspace index and version retention

## Purpose and scope

Two scale limits remain:

1. `Workspace::page`, `page_mut`, `children` and `breadcrumbs` scan the page
   vector. The sidebar tree calls `children` recursively, so rendering is
   quadratic as the workspace grows.
2. `CollabState::versions` grows without a limit; every edit appends a full
   page snapshot forever.

This spec adds a derived, private `WorkspaceIndex` and a per-page version
retention limit. The JSON shape and the public API stay unchanged.

Out of scope:

- a persistent database index (the JSON document is still the source of truth);
- block-level indexing;
- version diffs or compression;
- restoring from a version route on the server (the UI already restores through
  `PATCH`, which records a new version).

## Evidence

- `src/model.rs`: `page`, `page_mut`, `children`, `breadcrumbs` use
  `self.pages.iter()`.
- `src/ui/render/sidebar.rs`: `render_tree` calls `children` for each node.
- `src/collab.rs`: `record_version` appends without pruning.
- `docs/data-model.md` invariants I1 (unique ids) and I2 (valid parents).
- `spec/spec-016-architecture-follow-up-and-observation.md` A3/A4 plan this.

## Acceptance criteria

1. **Given** a workspace, **when** `page` or `page_mut` is called, **then** the
   lookup uses the derived id index and returns the same page as a linear scan.
2. **Given** a parent id, **when** `children` is called, **then** the child ids
   come from the index and map back to pages; roots use the index's root list.
3. **Given** a page id, **when** `breadcrumbs` is called, **then** the chain is
   built through indexed page lookups.
4. **Given** `add_page` or `remove_page`, **when** it succeeds, **then** the
   index is rebuilt and stays consistent with I1/I2.
5. **Given** a JSON document, **when** it is deserialized, **then** the index is
   rebuilt and the on-disk shape is unchanged.
6. **Given** a direct `pages` mutation that bypasses the workspace methods,
   **when** a lookup runs, **then** it falls back safely instead of returning a
   wrong page.
7. **Given** more than 50 versions for one page, **when** a new version is
   recorded, **then** the oldest versions for that page are pruned and other
   pages are unaffected.
8. **Given** a restore through `PATCH /api/pages/{id}`, **when** the update is
   recorded, **then** it creates a new version (audit trail).

## Test scenarios (written before implementation)

- `tests/model.rs`:
  - index lookups after `new`, `add_page`, `remove_page`;
  - `children` roots and nested children;
  - `breadcrumbs` through indexed lookups;
  - deserialized workspace still finds pages;
  - direct `pages.push` fallback still finds the page;
  - JSON round-trip equality is unchanged.
- `tests/collab.rs`:
  - 55 versions for one page keep the newest 50;
  - another page's versions are untouched;
  - `versions_for` remains newest-first.
- Existing tests, clippy, fmt and the screenshot stay green/unchanged.

## Design

- `model/index.rs`:
  - `WorkspaceIndex { positions: HashMap<String, usize>, children:
    HashMap<String, Vec<String>>, roots: Vec<String> }`;
  - `build(pages)`, `position(id)`, `children(parent)`, `len()`.
- `Workspace` gains a private `#[serde(skip)] index: WorkspaceIndex`.
  - custom `Deserialize` rebuilds the index;
  - manual `PartialEq`/`Eq` compare `name`, `pages`, `version` only;
  - `add_page` and `remove_page` rebuild it;
  - `page`/`page_mut` verify the indexed position and fall back to a linear
    scan when a direct `pages` mutation made the index stale.
- `CollabState::record_version` prunes the oldest versions for the same page
  beyond `MAX_VERSIONS_PER_PAGE` (50). Versions for other pages are untouched.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `WorkspaceIndex` + workspace accessors
- [x] custom serde/equality for `Workspace`
- [x] version retention constant + pruning
- [x] Docs, changelog, build/test/clippy/fmt/screenshot, commit
