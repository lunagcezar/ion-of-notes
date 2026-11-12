# spec-108 - Background remote request worker

- Status: implemented (2026-11-12)
- Related: `spec-044`, `spec-083`, `spec-101`
- Review source: ChatGPT performance analysis (remote REST on the UI thread)

## Purpose and scope

Remote mode used blocking `ureq` calls from the egui thread for search, page
saves, full-workspace replaces and CRDT sync/update. This spec introduces one
bounded background worker per remote connection:

- `RemoteRest` is a REST-only clone of `RemoteApi` (`base`, `token`, `agent`);
- `RemoteWorker` owns a `Client<RemoteRest>` on a plain thread;
- commands are `Search`, `SavePage`, `ReplaceWorkspace`, `CrdtSync` and
  `CrdtUpdate`; results are polled each frame;
- the command queue is bounded (256);
- search keeps at most one request in flight and one queued (latest wins);
- commits keep one plan in flight and coalesce newer edits into the next diff;
- CRDT sync/update is dispatched to the worker; a full queue falls back to the
  synchronous call so an update is not lost.

Local mode is unchanged and keeps the synchronous client.

Out of scope:

- concurrent workers or parallel HTTP requests;
- changing the REST/WebSocket protocol;
- server-side read snapshots (Phase 4).

## Acceptance criteria

P1. **Given** remote mode, **when** the user types, **then** the REST save runs
on the worker and the UI frame does not block on `ureq`.

P2. **Given** rapid search input, **when** several queries are typed, **then**
only the latest unfinished query is eventually requested.

P3. **Given** a commit in flight, **when** the page changes again, **then** the
newer diff is sent after the first result and the older snapshot never
overwrites the newer one.

P4. **Given** a remote CRDT update, **when** the worker queue has room, **then**
it is sent asynchronously; when full, the synchronous fallback preserves it.

P5. **Given** local mode, **when** search/commit/CRDT runs, **then** behavior is
unchanged.

P6. **Given** a remote `RemoteRest` client, **when** it reads the workspace,
gets a page and searches, **then** the REST contract is unchanged.

## Test scenarios

- `tests/remote.rs` (`remote_rest_client_roundtrips_without_the_websocket`)
- `tests/ui.rs` (`pending_edits_waiting_clears_force_but_keeps_the_change`)
- `tests/client.rs`
- `tests/ui_crdt.rs`
- Full quality gate

## Design

- One worker per connection; dropping the app drops the sender and stops it.
- `RemoteApi` keeps the WebSocket and delegates REST to `RemoteRest`.
- The UI owns all state changes; the worker only returns data.
- Search coalescing is explicit (`search_in_flight` + `search_queued`).
- Commit coalescing uses the existing `PendingEdits` diff, not a second queue.

## Plan / tasks

- [x] Extract `RemoteRest`
- [x] Add the bounded worker and commands/results
- [x] Route search, commits and CRDT through the worker
- [x] Add the REST client and pending-edit tests
- [x] Run the full gate and commit
