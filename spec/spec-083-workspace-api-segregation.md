# spec-083 - WorkspaceApi interface segregation

- Status: in progress (2026-11-12)
- Related: `spec-044`, `spec-049`
- Review sources: Claude and ChatGPT (fat `WorkspaceApi`, runtime-unavailable
  methods)

## Purpose and scope

`WorkspaceApi` combines request handling with CRDT, realtime and awareness
methods. Transports that only handle requests still have to satisfy the other
methods through defaults that return errors or `None`.

This spec splits the transport contract:

- `RequestTransport` owns `handle`;
- `CollabTransport` owns CRDT sync/update, realtime subscription and awareness;
- `WorkspaceApi` is the composite trait for clients that genuinely need both;
- `Client` request methods only require `RequestTransport`;
- CRDT/realtime client methods require the composite.

Scope:

- trait definitions and blanket composite;
- `WorkspaceServer`, `RemoteApi`, `DesktopApi` implementations;
- test stubs;
- `Client` impl bounds.

Out of scope:

- a separate `CollabClient` type;
- changing the remote protocol;
- splitting `Client` into resource modules (spec-084).

## Acceptance criteria

P1. **Given** a transport that only implements `RequestTransport`, **when** a
request client is constructed, **then** request methods compile and work.

P2. **Given** a transport that also implements `CollabTransport`, **when** the
client calls CRDT/realtime methods, **then** they work as before.

P3. **Given** an implementation missing a collab method, **when** it is compiled
against `CollabTransport`, **then** the compiler reports the missing method
instead of silently returning a runtime error.

P4. **Given** the existing remote and local modes, **when** the traits are
split, **then** all tests pass unchanged.

## Test scenarios

- `tests/client.rs` (request stub)
- `tests/security.rs` and `tests/ui.rs` (recording stubs)
- `tests/remote.rs` (remote request + collab)
- `tests/crdt.rs` and `tests/ui_crdt.rs` (collab path)

## Design

- `RequestTransport::handle` is the only request method.
- `CollabTransport` has four required methods; no runtime-unavailable defaults.
- `pub trait WorkspaceApi: RequestTransport + CollabTransport {}` plus a
  blanket impl keeps existing composite bounds readable.
- `Client<A: RequestTransport>` owns request methods; `Client<A: WorkspaceApi>`
  owns the collab methods.
- Test stubs implement both traits explicitly with `Err`/`None` where they do
  not support collaboration.

## Plan / tasks

- [x] Write spec-083 and adjust the traits
- [x] Update server, remote, desktop and test implementations
- [x] Split the `Client` impl bounds
- [x] Run the full gate and commit
