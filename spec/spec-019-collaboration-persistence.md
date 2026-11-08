# spec-019 — Collaboration persistence

## Purpose and scope

`WorkspaceServer` currently persists only `Workspace`. `CollabState`
(members, shares, sync log, page versions) lives in memory and is lost on
restart. This spec makes collaboration state durable through a new
`CollabStore` port and a JSON adapter, without changing the API contract.

Scope:

- a `CollabStore` port with `JsonCollabStore` and `NoCollabStore` adapters;
- a separate `collab.json` document beside `workspace.json`;
- loading/seeding collaboration state in the composition root;
- persisting collaboration state after member, share, page and workspace
  mutations;
- reporting collaboration-storage failures as API `500`;
- tests for round-trip, missing/corrupt files, restart restore and failures.

Out of scope:

- accounts/sessions/auth middleware (still staged behind ADR-003);
- PostgreSQL/SQLx or multi-process locking;
- merging concurrent server processes;
- schema versioning beyond serde defaults.

## Evidence

- `src/collab.rs`: `CollabState` is serde-friendly but only held in memory.
- `src/server.rs`: `WorkspaceServer::write` persists only the workspace;
  `/api/members` and `/api/shares` do not persist at all.
- `src/app.rs`: the composition root loads only the workspace JSON.
- `spec/spec-016-architecture-follow-up-and-observation.md` A1 plans this.
- `docs/adr-003-self-hosted-collaborative-api.md`: accounts, sharing, sync and
  version history are server-owned state.

## Acceptance criteria

1. **Given** a `CollabState` with members, shares, changes and versions,
   **when** it is persisted by `JsonCollabStore`, **then** loading it returns
   an equal state.
2. **Given** a missing `collab.json`, **when** the composition root starts,
   **then** the documented seed state is created and saved without touching
   `workspace.json`.
3. **Given** a corrupt `collab.json`, **when** loading, **then** the server
   falls back to the seed state instead of failing to start.
4. **Given** a server built with both paths, **when** a member is invited, a
   share is set, a page is created/updated and the server is rebuilt, **then**
   members, shares, changes and versions are restored.
5. **Given** a collaboration store whose `persist` fails, **when** a member or
   share mutation is handled, **then** the API returns `500` with an error
   envelope.
6. **Given** an existing workspace-only installation, **when** the app starts,
   **then** the workspace is preserved and collaboration state starts from the
   documented seed.

## Test scenarios (written before implementation)

- `tests/collab_store.rs` (new):
  - JSON collaboration round-trip.
  - Missing file seeds and saves.
  - Corrupt file falls back to the seed.
  - Server restart restores members, shares, changes and versions.
  - Failing collaboration store returns `500` for `POST /api/members` and
    `PUT /api/shares`.
- Existing `tests/server.rs`, `tests/store_port.rs`, `tests/integration.rs`
  must remain green.

## Design

- **Port:** `CollabStore { fn persist(&self, &CollabState) -> io::Result<()> }`
  in `src/store.rs`, next to `WorkspaceStore`.
- **Adapters:** `NoCollabStore` (null object) and `JsonCollabStore` (path).
- **Files:** `workspace.json` and `collab.json` in the same app-data directory.
  Keeping them separate means a corrupt collaboration document cannot destroy
  pages.
- **Loading:** `store::load_collab_or_seed(path)` returns `CollabState`; the
  composition root passes it to the server.
- **Server:** `WorkspaceServer` owns `collab_store`; `with_stores` and
  `with_paths` inject both stores. `write` persists both documents.
  Member/share routes persist collaboration state explicitly.
- **Errors:** a failed collaboration write is returned as `500`, matching the
  workspace-store behavior from spec-015.
- **Compatibility:** `WorkspaceServer::new` and `with_path` keep their current
  workspace-only behavior; the composition root uses `with_paths`.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `CollabStore` port + `JsonCollabStore` / `NoCollabStore`
- [x] `load_collab_or_seed` + `collab_path` helpers
- [x] `WorkspaceServer` holds and persists collaboration state
- [x] Composition root loads/seeds both documents
- [x] Member/share routes persist and report `500`
- [x] Docs, changelog, build/test/clippy/screenshot, commit
