# spec-074 - Atomic workspace and collaboration persistence

- Status: implemented (2026-11-12)
- Related: `spec-015`, `spec-039`
- Review sources: the 2026-11-12 external reviews (ChatGPT, Claude)

## Purpose and scope

The server persisted the workspace snapshot and the collaboration snapshot in
two separate writes. A failure between them could leave the durable documents
at different points, and the in-memory state could stay ahead of the store.

This spec adds an atomic snapshot port, a SQLite transaction implementation,
and a best-effort reload after a failed write.

Scope:

- `SnapshotStore` port with `persist_snapshots(workspace, collab)`;
- `WorkspaceStore::load` / `CollabStore::load` defaults and JSON/SQLite
  implementations;
- `WorkspaceServer::use_snapshot_store`;
- `SqliteStore::persist_snapshots` using one transaction;
- reload of the durable state after a failed `write` or CRDT projection;
- SQLite server wiring through `build_server_sqlite_at`;
- tests with a recording/failing snapshot store and a SQLite round-trip.

Out of scope:

- a normalized SQLite schema (pages/blocks/rows);
- cross-file atomicity for the JSON adapters (documented limitation);
- PostgreSQL.

## Acceptance criteria

P1. **Given** a server with an atomic snapshot store, **when** a mutation
succeeds, **then** both documents are committed through one
`persist_snapshots` call.

P2. **Given** a failing atomic store, **when** a mutation is attempted, **then**
the request reports a storage error and the in-memory workspace is reloaded
from the last durable snapshot.

P3. **Given** SQLite, **when** `persist_snapshots` runs, **then** workspace and
collaboration rows are written in one transaction and both load back.

P4. **Given** the SQLite server builder, **when** it constructs the server,
**then** it installs the same `SqliteStore` as the atomic snapshot store.

## Test scenarios

- `tests/persistence.rs::atomic_snapshot_store_receives_both_documents`
- `tests/persistence.rs::failed_snapshot_write_reloads_the_last_durable_state`
- `tests/persistence.rs::sqlite_persist_snapshots_roundtrips_both_documents`

## Design

- `SnapshotStore` is a separate port (Interface Segregation) so existing
  `WorkspaceStore`/`CollabStore` adapters keep working.
- `WorkspaceServer::persist` prefers the snapshot store when present; otherwise
  it writes the two ports in sequence.
- `SqliteStore::persist_snapshots` serializes both documents and executes two
  upserts inside `connection.transaction()`.
- `reload_after_failure` calls the `load` methods; adapters that cannot reload
  return `None` and the server keeps its current state.
- JSON remains a local single-process mode; two files cannot be committed
  atomically, so the docs call this out explicitly.

## Plan / tasks

- [x] Write spec-074 and failing persistence tests (red)
- [x] Add the `SnapshotStore` port and load defaults
- [x] Implement the SQLite transaction and server wiring
- [x] Add reload-after-failure
- [x] Add `docs/persistence-integrity.md`
- [x] Run the full gate and commit
