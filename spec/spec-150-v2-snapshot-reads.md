# spec-150 - V2 reads from an immutable snapshot

- Status: implemented (2026-11-25)
- Related: `spec-143`, `docs/performance-roadmap.md`
- Review source: follow-up to spec-143 (v2 reads still held the workspace mutex)

## Purpose and scope

Spec-143 released the lock for v1 snapshot reads, but the heavy v2 GET routes
still projected and serialized inside `WorkspaceServer::handle`. This spec gives
v2 the same immutable read snapshot.

### Snapshot cache

`WorkspaceSnapshot` holds `Arc<Workspace>`, `Arc<CollabState>` and both
versions. `CollabState::version` is bumped by `record`, so every committed write
invalidates the cache. `workspace_snapshot` reuses the cached pair until either
version changes, then clones both documents once while the caller holds the
lock.

### Fast path

For `GET /api/v2/workspace`, `/api/v2/pages`, `/api/v2/pages/{id}` and
`/api/v2/search`:

1. take the server lock;
2. run the normal v2 checks (`check_version`, `authenticate`, `check_runtime`,
   `check_capability`, `check_authorization`) and generate default data
   sources/views exactly like `handle_v2_inner`;
3. take the snapshot and release the lock;
4. project and serialize through `ReadView`;
5. take the lock again only to append the request-log entry.

All other v2 routes keep the existing path. The fast path returns the same
envelopes, statuses, projections, pagination, request ID and rate-limit
semantics as the generic handler, which the contract suite exercises.

## Acceptance criteria

P1. **Given** a v2 workspace read, **then** projection and JSON happen outside
the workspace lock.

P2. **Given** a write, **then** the next v2 read sees it; the cached snapshot is
invalidated by the version pair.

P3. **Given** a member, **then** `/api/v2/workspace`, `/api/v2/pages`,
`/api/v2/pages/{id}` and `/api/v2/search` return the same principal-aware
projection as before.

P4. **Given** missing/invalid version, token or capability, **then** the same
error envelope and request ID are returned.

P5. **Given** `page_size`/cursor, **then** pagination is unchanged.

## Test scenarios

- `tests/web.rs` (`v2_snapshot_reads_are_invalidated_by_writes` and the existing
  contract/projection suites).
- `tests/contract.rs`, `tests/pagination.rs`, `tests/smoke.rs`.
- `tests/authorization.rs` (v2 projections).

## Plan / tasks

- [x] Add `WorkspaceSnapshot` and the version-keyed cache.
- [x] Bump `CollabState::version` on every recorded change.
- [x] Add `prepare_v2_read` and the HTTP fast path.
- [x] Keep the request log and rate limiter semantics.
- [x] Add the invalidation/search regression test.
