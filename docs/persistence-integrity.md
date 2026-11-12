# Persistence integrity

This document describes how the server commits workspace and collaboration
state, what happens after a failed write, and how to back up and restore a
self-hosted SQLite deployment.

## 1. Two documents, one commit

The workspace snapshot and the collaboration snapshot are separate documents:

- workspace: pages, blocks, databases, views, settings;
- collaboration: members, shares, sync log, versions, webhook state.

`WorkspaceServer::persist` prefers an atomic `SnapshotStore` when one is
installed. `SqliteStore::persist_snapshots` serializes both documents and
writes them inside one SQLite transaction. Either both rows change or neither
does.

The SQLite server builder installs the same `SqliteStore` as the workspace
store, collaboration store, auth store, shared rate-limit store and atomic
snapshot store.

## 2. Failure behavior

- A failed write returns a storage error to the caller.
- `WorkspaceServer::reload_after_failure` reloads the durable snapshots when
  the adapters implement `load` (SQLite and JSON do).
- The in-memory workspace/collaboration state is restored from the last
  durable snapshot instead of staying ahead of the store.
- CRDT projection failures also reload and reset the page document.

The JSON adapters write each document through a temporary file and rename.
They cannot commit two files atomically; JSON mode is a local single-process
mode. Use SQLite for multi-user or remote deployments.

## 3. SQLite schema posture

The first SQLite slice stores the existing JSON documents in two rows
(`workspace`, `collab`) plus normalized tables for accounts, sessions and rate
limits. WAL mode is enabled and the connection is guarded by a process mutex.

This is a deliberate first step:

- simple deployment and backup;
- no separate database service;
- straightforward migration from the JSON documents.

Normalized page/block/row tables are a later spec. Until then, large workspace
writes serialize the full snapshot; measure before changing the schema.

## 4. Backup and restore runbook

### Backup

The Compose backup profile uses SQLite's online backup API. For a manual
backup with the server stopped:

```sh
sqlite3 data/ion-of-notes.sqlite3 ".backup 'backup/ion-of-notes-$(date +%Y%m%d).sqlite3'"
```

For a live server, use the Compose backup service or SQLite `.backup`; do not
copy the database file while WAL writes are in flight.

### Restore drill

1. Stop the server.
2. Copy the backup to a scratch path.
3. Open it with `sqlite3` and run `PRAGMA integrity_check;`.
4. Load both documents through the server (`GET /api/v2/workspace` with an
   owner session) and confirm the page count and the collaboration members.
5. Replace the live database only after the scratch restore is verified.
6. Start the server and check `/health`, `/ready` and one authenticated page
   read.

A backup that has never been restored is not a verified backup. Run the drill
on a schedule and record the date.

## 5. Operational notes

- Keep the database on a private volume; the production Compose stack uses the
  `ion-data` volume.
- The database contains password hashes and session hashes; treat it as a
  secret.
- The rate-limit table is shared by instances using the same file; SQLite is
  still a single-node store.
- A future PostgreSQL adapter should preserve the `SnapshotStore` transaction
  boundary and the `WorkspaceStore`/`CollabStore` ports.
