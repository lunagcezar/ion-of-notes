# spec-109 - Incremental search document cache

- Status: implemented (2026-11-12)
- Related: `spec-015`, `spec-104`, `spec-097`
- Review source: ChatGPT performance analysis (incremental search index)

## Purpose and scope

`Workspace::search` rebuilt every page's searchable body and lowercase text on
every query. This spec adds a runtime-only cache inside `Workspace`:

- `SearchDocument` stores the page title, lowercase title, body text and
  lowercase body;
- `SearchCacheCell` keeps documents by page id plus a dirty-page set;
- `page_mut` marks that page dirty, `block_mut` marks the block's page dirty,
  `add_page` marks the new page dirty and `remove_page` drops removed pages;
- the cache is skipped by serde and reset when a workspace is cloned, so undo
  snapshots do not duplicate every indexed body;
- equality ignores the cache.

The first search after deserialization indexes every page; later searches only
re-index pages that were mutated.

Out of scope:

- persisting the index;
- background index construction;
- fuzzy ranking.

## Acceptance criteria

P1. **Given** an unchanged workspace, **when** the same query runs twice, **then**
page bodies are not rebuilt on the second run.

P2. **Given** a block edit, **when** the page is searched, **then** the edited
page is re-indexed and the new text matches.

P3. **Given** a page title edit, **when** search runs, **then** the new title
matches with the title score.

P4. **Given** a removed page, **when** search runs, **then** it no longer
appears.

P5. **Given** a cloned or serialized workspace, **when** it is searched,
**then** results are correct and the cache is not serialized.

## Test scenarios

- `tests/model.rs` (`search_cache_tracks_page_block_and_remove_edits`,
  existing ranking/archive tests)
- `tests/server.rs` (`/api/search`)
- `tests/perf.rs`
- Full quality gate

## Design

- `SearchCacheCell(RefCell<SearchCache>)` provides interior mutability for
  `search(&self)`.
- `Clone` for the cell returns an empty cache.
- `WorkspaceData` deserialization initializes an empty cache.
- Ranking still iterates `workspace.pages`, so page order is unchanged.

## Plan / tasks

- [x] Add the document/cache types
- [x] Mark dirty pages from page/block/add/remove mutations
- [x] Reuse documents in `search`
- [x] Add the edit/remove tests
- [x] Run the full gate
