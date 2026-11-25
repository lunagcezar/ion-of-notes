# spec-151 - Publish read snapshots on write

- Status: implemented (2026-11-25)
- Related: `spec-143`, `spec-150`, `docs/performance-roadmap.md`
- Review source: remaining note from spec-150 (snapshot clone under the lock)

## Purpose and scope

Spec-150 cached an immutable snapshot but still cloned the workspace inside the
reader when the cache missed after a write. This spec publishes the snapshot on
the write path, so readers normally take only an `Arc`.

### Publication

- `WorkspaceServer` owns a shared `RwLock<Option<WorkspaceSnapshot>>`.
- `publish_snapshot` clones the workspace and collaboration state and stores the
  `Arc`-backed snapshot with both versions.
- `commit_write` publishes after a successful persist (REST writes are already
  debounced).
- `apply_crdt_update` publishes at most every 250 ms, so a fast typist does not
  clone a large workspace on every CRDT frame; a reader in that window falls
  back to the existing clone-on-miss path and stays correct.
- `reload_after_failure` publishes the reloaded state.
- Publishing is **opt-in**: `enable_read_snapshots` is called by the HTTP router.
  A purely in-process desktop server never pays the clone.

### Reads

The v1 and v2 fast paths ask the shared cell first. On a version match they clone
only the `Arc`s; projection and serialization stay outside the lock.

## Acceptance criteria

P1. **Given** the HTTP router, **then** a committed write publishes a snapshot
containing the change and the current version.

P2. **Given** a v1/v2 read after that write, **then** it uses the published
snapshot instead of cloning under the lock.

P3. **Given** an in-process server without HTTP, **then** writes do not clone for
snapshots.

P4. **Given** a burst of CRDT updates, **then** publication is throttled and
readers still see a consistent version (with a clone fallback in the window).

## Test scenarios

- `tests/web.rs` (`writes_publish_a_read_snapshot`,
  `v2_snapshot_reads_are_invalidated_by_writes` and the contract suites).
- `tests/authorization.rs` projection equivalence.
- `tests/perf.rs` continues to run without HTTP publication.

## Plan / tasks

- [x] Add the shared cell and opt-in switch.
- [x] Publish on REST writes, reload and throttled CRDT updates.
- [x] Wire the router to the server cell.
- [x] Add the publication test.
