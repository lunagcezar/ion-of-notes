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

The JSON adapters write each document through a temporary file and rename. If
the platform cannot replace an existing file, the previous document is moved to
a sibling `.bak` file until the new one is in place; the live file is never
overwritten with `fs::copy`. A corrupt or unreadable document is preserved and
the load returns an error instead of seeding a new workspace over it. JSON
files cannot commit two documents atomically; JSON mode is a local
single-process mode. Use SQLite for multi-user or remote deployments.

Collaboration state follows the same rule as the workspace: only a missing file
is seeded. A corrupt, unreadable or permission-denied `collab.json` makes
`load_collab_or_seed` return the error and leaves the bytes untouched, and
`WorkspaceServer::with_paths` fails closed instead of starting with empty
members/shares.

### Webhook outbox

Webhook deliveries are part of the collaboration document. The outbox is
considered durable only after a successful persist. When a persist fails, the
worker stops sending (`prepare_webhook_deliveries` returns no jobs) until a
later persist succeeds, so a delivery is never sent before the server could
replay it. Retention removes delivered or dead-lettered records only; if the
outbox is full of pending records, new deliveries are refused and the events
stay queued (bounded at `MAX_WEBHOOK_QUEUE`) instead of dropping a pending
delivery. Delivery is therefore at-least-once.

## 3. SQLite schema posture

The SQLite adapter stores workspace metadata in `workspace_meta`, pages in
`workspace_pages`, blocks in `workspace_blocks`, collaboration state in
`collab`, and accounts/sessions/rate limits in their own tables. WAL mode,
foreign keys and a busy timeout are enabled; the connection is guarded by a
process mutex.

- Page rows carry metadata only; block rows carry `page_id`, `parent_id`,
  `position` and the block JSON without children.
- A single-block PATCH/DELETE writes one block row, the page row and the
  collaboration row in one transaction.
- Full snapshots and page/children writes replace the affected block rows.
- Opening an older database splits page JSON blocks into rows once.

Large workspace metadata (databases, data sources, views, comments and uploads)
is still one JSON document; measure before normalizing it further.

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
