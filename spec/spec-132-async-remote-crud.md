# spec-132 - Asynchronous remote CRUD

- Status: implemented (2026-11-24)
- Related: `spec-044`, `spec-055`, `spec-108`, `spec-131`
- Review source: deferred asynchronous remote CRUD item

## Purpose and scope

The desktop remote transport already ran search, page saves, workspace
replaces and CRDT on the background `RemoteWorker`. Page create/delete, member
invites, permission updates and version restores still called the blocking
`ureq` client on the egui thread.

This spec routes those operations through the existing bounded worker:

- new `RemoteCommand` variants: `CreatePage`, `DeletePage`, `InviteMembers`,
  `SetShare`;
- matching `RemoteResult` variants;
- the worker executes them through `Client<RemoteRest>` and returns owned
  results;
- the UI dispatches from `NewPage`, `DeletePage`, `SendInvites`,
  `SetGeneralAccess` and `RestoreVersion` when in remote mode, falling back to
  the synchronous path only when the bounded queue is unavailable/full;
- `NewPage` carries `open_in_new_tab` so the result selects the right tab;
- invite and share results reuse `finish_invites`/`finish_share`, shared with
  the local path.

Out of scope:

- a loading spinner per operation (the UI still uses the existing status line);
- cancelation of an in-flight request;
- batching unrelated resource families.

## Acceptance criteria

P1. **Given** remote mode, **when** the user creates a page, **then** the
blocking client is not called on the UI thread and the created page opens on
the result.

P2. **Given** remote mode, **when** the user deletes a page, **then** the
request is queued and a success triggers the existing workspace reload.

P3. **Given** a batch of invite emails, **then** one worker command returns the
invited count and per-address errors.

P4. **Given** a permission change, **then** the worker returns the grant and
the UI applies the same status/general-access update as local mode.

P5. **Given** a full worker queue, **then** the command falls back to the
existing synchronous path rather than dropping it silently.

## Test scenarios

- `tests/remote.rs` (`remote_crud_operations_roundtrip`, existing remote
  REST/CRDT/WebSocket tests).
- `tests/ui.rs` (UI helper compilation and existing state tests).
- `cargo test --features desktop --test ui --test fidelity`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Commands/results | `ui/remote_worker.rs` |
| Dispatch/results | `ui/apply/remote.rs` |
| Create/delete commands | `ui/apply/navigation.rs`, `ui/apply/tabs.rs` |
| Invite/share commands | `ui/apply/sharing.rs` |
| Version restore | `ui/apply/page_actions.rs` |

## Plan / tasks

- [x] Add create/delete/invite/share commands and results.
- [x] Dispatch them from the UI command handlers.
- [x] Apply results on the UI thread, preserving tab/history behavior.
- [x] Keep a synchronous fallback for a full queue.
- [x] Add the remote CRUD round-trip test.
