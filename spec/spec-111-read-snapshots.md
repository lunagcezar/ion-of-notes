# spec-111 - Immutable read snapshots for v1 reads

- Status: implemented (2026-11-12)
- Related: `spec-038`, `spec-074`, `spec-101`
- Review source: ChatGPT performance analysis (single global server mutex)

## Purpose and scope

The Axum adapter serialized every request through one
`Mutex<WorkspaceServer>`, including the full-workspace GET that serializes the
largest response. This spec adds an immutable workspace snapshot for the
expensive read-only v1 routes:

- `GET /api/workspace`
- `GET /api/pages`
- `GET /api/pages/{id}`

The request still takes the server lock briefly to authenticate and, for a
page, to resolve access. It then clones or reuses an `Arc<Workspace>` snapshot
and serializes outside the lock. The snapshot is keyed by `Workspace::version`
and invalidated after any non-GET request or a WebSocket CRDT update.

Writes remain ordered through the existing `Mutex<WorkspaceServer>`; v2 stays
on the locked pipeline because its capability, limit and request-ID policy
must run against the live server.

Out of scope:

- concurrent v2 reads;
- a separate writer actor;
- caching auth/session lookups;
- changing the response envelopes.

## Acceptance criteria

P1. **Given** an authenticated v1 workspace read, **when** the snapshot version
matches, **then** the response is serialized from the snapshot without holding
the server lock.

P2. **Given** a write, **when** it completes, **then** the next workspace read
rebuilds the snapshot and reflects the write.

P3. **Given** an auth-required server, **when** a snapshot read has no token,
**then** it returns `401` exactly like the locked path.

P4. **Given** a page read, **when** the connection cannot read that page, **then**
it returns `403`.

P5. **Given** a WebSocket CRDT update, **when** it applies, **then** the snapshot
is invalidated.

P6. **Given** the existing web/realtime tests, **when** the fast path is used,
**then** statuses, envelopes and bodies are unchanged.

## Test scenarios

- `tests/web.rs` (`snapshot_reads_stay_correct_after_a_write`,
  `snapshot_reads_require_auth_when_configured`, existing route tests)
- `tests/realtime.rs`
- `tests/smoke.rs`
- Full quality gate

## Design

- `AppState` owns `Arc<RwLock<Option<Arc<Workspace>>>>`.
- The snapshot is cloned under the server lock once per workspace version.
- `authenticate` mirrors the v1 auth branch (`requires_auth` or owner token).
- The page route checks `can_read_page` before reading the snapshot.
- Invalidation is cheap: `*snapshot = None` after a non-GET request or CRDT
  update.

## Plan / tasks

- [x] Add the snapshot cache and auth helper
- [x] Serve the three v1 read routes from it
- [x] Invalidate after writes and WebSocket CRDT updates
- [x] Add the write/auth tests
- [x] Run the full gate and commit
