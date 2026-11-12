# spec-114 - Incremental SQLite page persistence

- Status: implemented (2026-11-12)
- Related: `spec-039`, `spec-074`, `spec-097`
- Review source: ChatGPT performance analysis (full JSON snapshots in SQLite)

## Purpose and scope

SQLite stored the whole workspace as one JSON document, so every one-page write
re-serialized and rewrote every page. This spec normalizes the page level while
keeping the rest of the workspace as one small metadata document:

- `workspace_meta` holds name, version, settings, databases, data sources,
  views, comments and file uploads;
- `workspace_pages` holds one row per page (`id`, `position`, `json`);
- `SnapshotStore::persist_page` updates one page, refreshes positions and writes
  collaboration state in one transaction;
- `WorkspaceServer::write` and `apply_crdt_update` call `persist_page` instead
  of the full snapshot;
- full persists still use `persist_snapshots` (replace, import, tests);
- the legacy `workspace` JSON table is read once and migrated on `open`;
- loading rebuilds the workspace from metadata plus ordered page rows.

Out of scope:

- per-block or per-property tables;
- an operation log;
- changing the JSON store adapter;
- PostgreSQL.

## Acceptance criteria

P1. **Given** a SQLite server, **when** one page changes, **then** only that
page row plus metadata and collaboration state are written.

P2. **Given** a page deletion, **when** it persists, **then** its row is
removed and the remaining order is unchanged.

P3. **Given** a page reorder, **when** it persists, **then** positions are
refreshed and loading preserves the order.

P4. **Given** a legacy database with a `workspace` JSON row, **when** it is
opened, **then** the row is migrated into `workspace_meta` and
`workspace_pages` and loads identically.

P5. **Given** the existing SQLite/auth/integration tests, **when** the schema
changes, **then** round-trips and restarts still pass.

## Test scenarios

- `tests/sqlite_store.rs`
  (`workspace_and_collab_roundtrip_through_sqlite`,
  `incremental_page_persist_updates_one_page_and_preserves_order`,
  `empty_sqlite_database_migrates_json_documents`)
- `tests/auth.rs` (SQLite session restart)
- `tests/integration.rs`
- `tests/smoke.rs`
- Full quality gate

## Design

- `WorkspaceMeta` is a small serializable view of the non-page fields.
- `Workspace::from_stored_parts` rebuilds the derived index and an empty search
  cache.
- `persist_page` uses one transaction: metadata, one page upsert/delete,
  position refresh and collaboration state.
- The old `workspace` table remains for backward-compatible migration only.

## Plan / tasks

- [x] Add the metadata and page tables
- [x] Add migration from the legacy JSON row
- [x] Add `persist_page` and wire the server write path
- [x] Add incremental/order/delete tests
- [x] Run the full gate and commit
