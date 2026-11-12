# spec-085 - Transport adapter split

- Status: implemented (2026-11-12)
- Related: `spec-038`, `spec-044`, `spec-083`, `spec-084`
- Review sources: Claude and ChatGPT (transport modules with several
  responsibilities)

## Purpose and scope

`server/web.rs` combined router construction, HTTP translation, health/readiness,
metrics and the WebSocket session. `remote.rs` combined session persistence,
login, REST execution, CRDT methods and the background WebSocket.

This spec splits both adapters by responsibility while keeping the public
paths and behavior unchanged.

Scope:

- `server/web/`:
  - `mod.rs`: router, state, server startup, webhook worker, tracing/shutdown;
  - `http.rs`: HTTP request translation and response helpers;
  - `operations.rs`: health, readiness and metrics;
  - `websocket.rs`: WebSocket upgrade/session and CRDT message handling.
- `remote/`:
  - `mod.rs`: `RemoteApi`, login, request/CRDT trait implementations;
  - `session.rs`: remembered session persistence;
  - `http.rs`: ureq agent, GET/execute helpers;
  - `realtime.rs`: background WebSocket and reconnection;
  - `crdt.rs`: CRDT transport methods.

Out of scope:

- changing the REST/WebSocket protocol;
- changing session storage;
- a new transport.

## Acceptance criteria

P1. **Given** `crate::server::web::router` and `serve_axum`, **when** the
adapter is split, **then** the public paths and behavior are unchanged.

P2. **Given** the health/ready/metrics routes, **when** they are called, **then**
the same responses and content types are returned.

P3. **Given** the WebSocket CRDT path, **when** a client connects, **then**
ready, backlog, live changes and CRDT sync still work.

P4. **Given** a remembered remote session, **when** it is saved, loaded or
cleared, **then** the file format and permissions are unchanged.

P5. **Given** the remote REST and realtime tests, **when** the split is applied,
**then** they pass unchanged.

## Test scenarios

- `tests/web.rs`
- `tests/smoke.rs`
- `tests/remote.rs`
- `tests/realtime.rs` and `tests/crdt.rs`
- Full quality gate

## Design

- The module root keeps only composition and shared state.
- Submodules use `use super::*;` and `pub(super)` for the few helpers the
  parent needs.
- No protocol, route or DTO changes.

## Plan / tasks

- [x] Write spec-085
- [x] Split `server/web` and `remote`
- [x] Run the web/smoke/remote/realtime tests
- [x] Run the full gate and commit
