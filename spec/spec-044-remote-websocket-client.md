# spec-044 — Remote WebSocket client transport

## Purpose and scope

The desktop only talks to the in-process `WorkspaceServer`. This spec adds a
remote `WorkspaceApi` transport: blocking REST through ureq and a background
WebSocket for realtime changes and awareness. The desktop can now connect to a
running `--serve` instance with `--server` and `--token`.

In scope:

- `RemoteApi` implementing `WorkspaceApi` over HTTP;
- background WebSocket thread with reconnect and `?since=` resume;
- typed realtime/awareness forwarding to the UI;
- REST CRDT sync/update endpoints for remote clients;
- `DesktopApi` local/remote enum and `--server`/`--token` CLI wiring;
- tests against a real Axum server.

Out of scope:

- a login UI (the token is supplied on the command line);
- TLS certificate/private-CA configuration;
- WebSocket compression, binary frames or multi-node fan-out;
- offline queueing of REST requests while the server is down.

## Evidence

- `src/server/web.rs` — Axum HTTP/WebSocket adapter.
- `src/server/realtime.rs` — typed messages and hub.
- `src/server.rs` — in-process CRDT methods.
- `src/client.rs` — typed client over `WorkspaceApi`.
- `spec/spec-040` … `spec/spec-043`.

## Acceptance criteria

1. **Given** `--server http://host:port --token <token>`, **when** the desktop
   starts, **then** it loads the workspace through the remote REST API.
2. **Given** a remote workspace, **when** v1/v2 calls run, **then** they use
   ureq and preserve headers, status, body and request ID.
3. **Given** a remote CRDT sync/update, **when** the client calls it, **then**
   the server merges through the REST CRDT routes.
4. **Given** the remote WebSocket, **when** a change or awareness event occurs,
   **then** the UI receiver gets the typed message.
5. **Given** a dropped socket, **when** it reconnects, **then** it resumes with
   the last seen sequence via `?since=`.
6. **Given** no `--server`, **when** the desktop starts, **then** the local
   in-process mode is unchanged.
7. **Given** `--server` without `--token`, **when** the desktop starts, **then**
   it fails with a clear error.
8. **Given** the existing suite, **when** it runs, **then** all tests stay green
   and the new `tests/remote.rs` passes.

## Test scenarios (written before implementation)

- `tests/remote.rs` (new):
  - remote REST workspace/create-page and CRDT sync/update;
  - remote WebSocket ready, change and awareness;
  - reconnect/resume behavior is exercised by the `since` cursor.
- `tests/cli.rs`: `--server` and `--token` parsing.
- Existing local `tests/app.rs`, `tests/ui_crdt.rs` stay green.

## Design

```text
Desktop NotionApp
  └── Client<DesktopApi>
          ├── DesktopApi::Local(Box<WorkspaceServer>)
          └── DesktopApi::Remote(RemoteApi)
                  ├── ureq Agent  -> REST v1/v2 + /api/v2/crdt/*
                  └── WS thread   -> /ws/workspace/{id}?token&since
                          └── tokio broadcast -> UI receiver
```

- `RemoteApi` keeps the base URL, token and a ureq agent.
- REST errors are returned as `ApiResponse` statuses; network failures become a
  `503` envelope so the existing client error mapping applies.
- CRDT sync/update use JSON DTOs with base64 Yrs payloads.
- A background thread owns a current-thread Tokio runtime and the WebSocket;
  it reconnects with exponential backoff and tracks the last sequence.
- `DesktopApi` keeps the UI type stable; `build_desktop` stays local and
  `build_remote_desktop` is the remote composition.
- `main` requires both `--server` and `--token` for remote mode.

## Plan / tasks

- [x] Spec + failing tests (red).
- [x] REST CRDT DTOs/routes.
- [x] `RemoteApi` REST + WebSocket thread.
- [x] `DesktopApi`, composition and CLI.
- [x] Tests and docs.
- [x] Build/test/clippy/fmt/docs/screenshot, commit.

## Result

- Added `src/remote.rs`: `RemoteApi` with a ureq REST client and a background
  `tokio-tungstenite` socket that reconnects with `?since=`.
- Added REST CRDT routes `POST /api/v2/crdt/sync` and
  `POST /api/v2/crdt/update` with base64 Yrs payloads.
- Added `DesktopApi::{Local, Remote}`; `build_remote_desktop` and
  `NotionApp::with_client` keep the UI type stable.
- Added `--server` and `--token`; remote mode requires both.
- Remote realtime messages and awareness flow through the same broadcast
  receiver used by the in-process mode.
- Tests: 2 in `tests/remote.rs` plus CLI parsing; the suite is 251 tests.
- Added `docs/adr-013-remote-websocket-client.md`.

## Requirement mapping

| Requirement | How |
| ----------- | --- |
| FR (remote client) | `RemoteApi` + `DesktopApi::Remote` |
| FR (realtime) | background WS with `?since=` |
| NFR (compatibility) | local mode unchanged |
| NFR (testability) | real Axum server tests |
