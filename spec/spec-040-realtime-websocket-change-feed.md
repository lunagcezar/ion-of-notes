# spec-040 — Real-time WebSocket change feed and presence

## Purpose and scope

`GET /api/realtime` has described the decided WebSocket channel as data, but no
socket runs. This spec adds the first real-time transport slice: an authenticated
WebSocket endpoint, a versioned change feed backed by the existing sync log,
resume by cursor, and presence. It intentionally stages full CRDT merge for a
later spec; the current server already versions pages and supports `If-Match`,
so clients can reconcile page snapshots safely.

In scope:

- `GET /ws/workspace/{workspace_id}` with Bearer header or `?token=`;
- `ready`, `change`, `presence`, `pong` and `error` messages;
- `?since=<seq>` backlog replay from `collab.changes`;
- broadcast of every successful `WorkspaceServer::write` change;
- presence join/leave for active sockets;
- ping/pong heartbeat;
- `RealtimeHub` and protocol types in `src/server/realtime.rs`;
- tests for auth, ready, backlog, live change, presence and ping.

Out of scope:

- CRDT merge (Yrs/Automerge), per-block operations and awareness payloads;
- client UI integration and a Rust WebSocket client;
- multi-node fan-out, Redis/NATS and sticky sessions;
- binary frames, compression and backpressure tuning;
- changing v1/v2 REST behavior.

## Evidence

- `docs/adr-003-self-hosted-collaborative-api.md` — `WS /ws/workspace/{id}`.
- `src/collab.rs` — `Change`, `ChangeOp`, `SyncBatch`, `RealtimeContract`.
- `src/server.rs` — `write` records one change and persists it.
- `src/server/web.rs` — Axum adapter (spec-038).
- `src/auth.rs` — sessions and the demo owner token (spec-039).

## Acceptance criteria

1. **Given** a valid session or owner token, **when** the client connects to
   `/ws/workspace/{id}`, **then** the socket upgrades and receives `ready`.
2. **Given** a missing or invalid token, **when** the client connects, **then**
   the upgrade fails with `401`.
3. **Given** a workspace id that does not match the server, **when** the client
   connects, **then** the upgrade fails with `404`.
4. **Given** `?since=<seq>`, **when** the socket opens, **then** every change
   with a greater sequence is replayed in order before live events.
5. **Given** a successful REST mutation, **when** the server writes the change,
   **then** every connected socket receives a `change` message with the same
   `seq`, `page_id`, `op` and `actor`.
6. **Given** two sockets, **when** one connects or disconnects, **then** the
   other receives an updated `presence` list.
7. **Given** a client `ping`, **when** the server handles it, **then** it
   replies with `pong`.
8. **Given** `GET /api/realtime`, **when** it is read, **then** it describes the
   socket URL, message names, resume cursor and presence.
9. **Given** the existing suite, **when** it runs, **then** all tests stay green
   and the new `tests/realtime.rs` passes.
10. **Given** the dependency direction, **when** the socket is added, **then**
    no WebSocket type appears in `model`, `client` or `collab`.

## Test scenarios (written before implementation)

- `tests/realtime.rs` (new):
  - hub publish/subscribe and presence;
  - missing token → HTTP 401; wrong workspace → 404;
  - ready + ping/pong + presence over a real socket;
  - backlog replay with `?since=`;
  - live change after a REST mutation through the shared server;
  - `GET /api/realtime` contract fields.
- Existing `tests/contract.rs`, `tests/web.rs` and `tests/server.rs` stay green.

## Design

```text
WorkspaceServer::write
   -> collab.record(Change)
   -> persist
   -> RealtimeHub.publish_change(Change)
          -> broadcast::Sender
                -> WebSocket session(s)

GET /ws/workspace/{id}?token=&since=
   -> auth: Bearer header or query token
   -> workspace id check
   -> ready { workspace_id, session_id, seq }
   -> backlog changes > since
   -> live broadcast + presence + ping/pong
```

- `src/server/realtime.rs` owns `RealtimeHub`, `ServerMessage`,
  `ClientMessage` and `PresenceMember`.
- `RealtimeHub` uses a Tokio broadcast channel and a presence map.
- `WorkspaceServer` owns a hub; `write` publishes the recorded change after a
  successful persist.
- `src/server/web.rs` adds the WebSocket route and keeps the REST fallback.
- Auth reuses `connection_for_token`, so sessions and the demo owner token both
  work.
- The protocol is JSON text frames with a `type` discriminator.

## Plan / tasks

- [x] Spec + failing tests (red).
- [x] Add Axum `ws`, Tokio `time`/`sync`, dev WebSocket client dependencies.
- [x] `RealtimeHub`, protocol types and `write` broadcast.
- [x] WebSocket route, auth, backlog, presence and ping/pong.
- [x] Update `RealtimeContract` and `GET /api/realtime`.
- [x] Tests and docs.
- [x] Build/test/clippy/fmt/docs/screenshot, commit.

## Result

- Added `src/server/realtime.rs`: `RealtimeHub` (Tokio broadcast + presence
  map), `ServerMessage`, `ClientMessage` and `PresenceMember`.
- `WorkspaceServer::write` now publishes the committed `Change` after a
  successful persist; `WorkspaceServer::realtime()` exposes the hub.
- Added `GET /ws/workspace/{id}` with Bearer or `?token=` auth, workspace
  matching, `?since=` backlog replay, `ready`/`change`/`presence`/`pong`
  messages and ping/pong handling.
- `RealtimeContract` now describes the socket URL, resume cursor, message names
  and the staged CRDT merge.
- Added `tests/realtime.rs`: 6 tests for hub broadcast, auth/workspace
  rejection, ready/presence/pong, backlog replay, live change and contract.
- Full CRDT merge (Yrs/Automerge) remains a follow-up; the running slice is a
  versioned change feed plus presence.
- The suite is 237 tests.

## Requirement mapping

| Requirement | How |
| ----------- | --- |
| FR (real-time transport) | authenticated `/ws/workspace/{id}` |
| FR (resume) | `?since=` replay from the change log |
| FR (presence) | join/leave presence broadcasts |
| NFR (observability) | request/session IDs and typed messages |
| NFR (compatibility) | REST paths and existing auth unchanged |
