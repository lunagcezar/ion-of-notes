# spec-133 - Per-block SQLite persistence

- Status: implemented (2026-11-24)
- Related: `spec-039`, `spec-114`, `spec-129`
- Review source: deferred per-block persistence item

## Purpose and scope

SQLite stored one page JSON row, so a block edit rewrote every block in that
page. Pages with thousands of blocks paid O(page) per edit.

This spec normalizes blocks into `workspace_blocks`:

- `workspace_blocks(id, page_id, parent_id, position, json, updated_at)` with an
  index on `(page_id, parent_id, position)`;
- page rows keep page metadata only; blocks are written with their children
  cleared and `parent_id`/`position` for tree reconstruction;
- `SnapshotStore::persist_block` writes one block row plus page metadata and
  collaboration state;
- `WorkspaceServer::write_block` is used by single-block PATCH/DELETE; page and
  children mutations keep the page write;
- full snapshots/page writes replace all block rows for the affected page;
- `open` migrates legacy page JSON with blocks into block rows once;
- `load_workspace` rebuilds nested trees from rows, with cycle protection.

Out of scope:

- per-block version history;
- data-source row blocks (they remain inside the workspace metadata JSON);
- a block-diff operation log.

## Acceptance criteria

P1. **Given** a SQLite server, **when** one block is patched, **then** one
`workspace_blocks` row plus page metadata/collab are written.

P2. **Given** nested children, **when** the workspace is reloaded, **then** the
tree is reconstructed with the same order and ids.

P3. **Given** legacy page JSON that still contains blocks, **when** the database
is opened, **then** blocks move to rows and the page row loses its block array.

P4. **Given** an unknown/data-source row id, **then** `persist_block` does not
delete a `workspace_pages` row.

P5. **Given** a deleted block, **then** its row and descendant rows are removed.

## Test scenarios

- `tests/sqlite_store.rs`
  (`block_persist_updates_one_row_and_roundtrips_nested_children`,
  `page_json_with_blocks_is_migrated_into_block_rows`,
  `v2_block_patch_uses_the_block_write_path`,
  existing page/workspace round-trips).
- `tests/blocks.rs` (block API behavior).
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Schema/migration | `store/sqlite.rs` |
| Block tree read/write | `store/sqlite.rs` |
| Port | `SnapshotStore::persist_block` |
| Server write path | `server.rs::write_block` |
| Block handlers | `server/handlers/blocks.rs` |

## Plan / tasks

- [x] Add the block table and index.
- [x] Serialize page metadata without blocks and blocks without children.
- [x] Rebuild nested trees on load with cycle protection.
- [x] Add `persist_block` and the server `write_block` path.
- [x] Route single-block PATCH/DELETE through it.
- [x] Migrate existing page JSON.
- [x] Add round-trip, migration and API tests.
