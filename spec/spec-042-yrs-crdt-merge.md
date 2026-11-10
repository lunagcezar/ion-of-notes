# spec-042 — Yrs CRDT text merge

## Purpose and scope

The real-time channel streams page-level changes, but two clients editing the
same block still rely on whole-page writes and `If-Match`. This spec adds a
server-side Yrs text CRDT per page: clients sync a state vector, exchange
compact updates, and the server projects the merged title/block text back into
the durable page snapshot.

In scope:

- `CrdtStore` with one Yrs `Doc` per page;
- title text and one text type per block id;
- `crdt_sync` (state vector → update + current vector);
- `apply_crdt_update` (merge, project, persist, record a sync change);
- WebSocket `crdt_sync` / `crdt_update` messages;
- REST writes reset the CRDT document so structure remains REST-owned;
- convergence tests with two Yrs clients and a WebSocket round-trip.

Out of scope:

- wiring Yrs into the egui editor (a client-side adapter is a later spec);
- CRDT for block structure, properties, comments or databases;
- persistent CRDT history/undo; the durable page snapshot is the projection;
- awareness/cursor payloads, offline replay and multi-node fan-out.

## Evidence

- `src/server/realtime.rs` — WebSocket change feed (spec-040).
- `src/model.rs` — `Page`, `Block`, `RichText`.
- `src/server.rs` — `write` and persistence.
- `yrs` 0.28 — `Doc`, `Text`, `StateVector`, `Update`.
- ADR-009 — change-feed-first decision; CRDT staged.

## Acceptance criteria

1. **Given** a page, **when** `crdt_sync` is called without a state vector,
   **then** it returns a full update and the current state vector.
2. **Given** two clients with the same initial update, **when** both insert at
   different positions and exchange updates, **then** their text converges.
3. **Given** a CRDT update, **when** the server applies it, **then** the page's
   title/block text is projected into the workspace snapshot and persisted.
4. **Given** a WebSocket `crdt_sync`, **when** the server responds, **then** the
   client receives a base64 update and state vector.
5. **Given** a WebSocket `crdt_update`, **when** it is applied, **then** all
   subscribers receive the update and the server snapshot reflects the merge.
6. **Given** a REST write, **when** it completes, **then** the CRDT document is
   reset and the next sync re-seeds from the new page snapshot.
7. **Given** a restart, **when** the page is synced, **then** the CRDT doc is
   re-seeded from the persisted projected text.
8. **Given** the existing suite, **when** it runs, **then** all tests stay green
   and the new `tests/crdt.rs` passes.

## Test scenarios (written before implementation)

- `tests/crdt.rs` (new):
  - two-client convergence and server projection;
  - WebSocket `crdt_sync` + `crdt_update` convergence;
  - CRDT sync reachable through the server API;
  - REST write reset (covered by projection/re-seed behavior).
- Existing `tests/realtime.rs`, `tests/webhooks.rs` and `tests/server.rs` stay
  green.

## Design

```text
Client A ── crdt_sync(page, sv) ──> Server CrdtStore (Yrs Doc)
Client B ── crdt_update(page, u) ─> apply -> project -> persist
                                     └── broadcast crdt_update
```

- `src/server/crdt.rs` owns `CrdtStore`.
- One `Doc` per page; root texts are `"title"` and `"block:<id>"`.
- `seed` creates text refs before opening a write transaction (Yrs locks are
  not re-entrant).
- `project` writes merged title/block text back; a single plain rich-text
  object is rebuilt when the block has at most one segment.
- `WorkspaceServer::crdt_sync` and `apply_crdt_update` are the application
  methods; `write` resets the page document after REST mutations.
- WebSocket messages carry base64-encoded Yrs updates.
- CRDT history is in memory; the page snapshot is the durable projection, so a
  restart re-seeds from the merged text.

## Plan / tasks

- [x] Spec + failing tests (red).
- [x] Add `yrs` and `base64`.
- [x] `CrdtStore` seed/sync/apply/project/reset.
- [x] Server methods and REST reset.
- [x] WebSocket CRDT messages.
- [x] Tests and docs.
- [x] Build/test/clippy/fmt/docs/screenshot, commit.

## Result

- Added `src/server/crdt.rs`: one Yrs `Doc` per page, `"title"` and
  `"block:<id>"` text types, state-vector sync, update merge, projection and
  reset.
- `WorkspaceServer::crdt_sync` and `apply_crdt_update` merge text, project it
  into the workspace snapshot, record a sync change and persist.
- REST writes reset the page's CRDT document so structure stays REST-owned.
- WebSocket messages now include base64 `crdt_sync` and `crdt_update`; merged
  updates broadcast to all subscribers.
- Added `tests/crdt.rs`: 3 tests for two-client convergence, server projection,
  WebSocket sync/update and API reachability.
- CRDT history is in memory; the persisted page snapshot is the durable
  projection and re-seeds after restart.
- Added `docs/adr-011-yrs-crdt-merge.md`.
- The suite is 243 tests.

## Requirement mapping

| Requirement | How |
| ----------- | --- |
| FR (CRDT merge) | Yrs text per page and block |
| FR (transport) | WebSocket `crdt_sync`/`crdt_update` |
| NFR (durability) | projected page snapshot persisted after each update |
| NFR (compatibility) | REST remains structure source; CRDT resets on writes |
