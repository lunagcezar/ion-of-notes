# Performance and scale roadmap

The 2026-11-12 reviews identified performance and scale items that are real
but larger than the security hardening session. This document records the
current posture, the deferred work and the measurements needed before changing
the architecture.

## 1. Current posture

- The desktop local mode is in-process and synchronous; it does not cross a
  network boundary.
- The remote desktop client sends CRUD and CRDT work through a bounded worker;
  when the queue is full, CRDT updates wait in a pending queue instead of
  blocking the UI (spec-132, spec-141).
- Workspace and collaboration snapshots are serialized as JSON in the JSON
  adapter. SQLite commits both in one transaction and normalizes pages and
  blocks into rows (spec-074, spec-133); only workspace metadata and the
  collaboration document remain JSON in their tables.
- Yrs CRDT covers page titles, block text and structure/properties/database row
  registers (spec-134, spec-135).
- Typing sends only the transaction update and local mode persists through the
  debounced REST write; the sidebar/page are not rebuilt behind the icon picker
  (spec-141).
- The Axum adapter runs the synchronous server in `spawn_blocking` behind one
  mutex. v1 and the heavy v2 GETs project and serialize from a versioned snapshot
  outside the lock; HTTP writes publish it. Writes still serialize (spec-143,
  spec-150, spec-151).
- The in-memory request log is capped; resolved sessions are cached for 30 s
  (`spec-145`), so most requests do not touch the auth store.
- SQLite updates one page row and one block row per edit and skips the position
  rewrite unless the order changed (spec-114, spec-116, spec-133).
- The in-memory rate limiter purges expired windows at a bound; the shared
  limiter has a local contingency.
- Webhook events queue in the persisted outbox and the worker sends outside the
  workspace lock (spec-117).
- View/data-source queries materialize only the requested window (spec-118),
  and the renderer jumps to the visible block window through a height index
  (spec-119).

## 2. Deferred work

The 2026-11-12 pass removed the P0 frame-wide clone/diff, normalized emoji
filtering, indexed remote cursors by block and removed duplicate search-body
work (specs 101-104; see `docs/performance-improvements.md`). The items below
remain staged.
| Item | Why it is deferred | Suggested next step |
| ---- | ------------------ | ------------------- |
| Asynchronous remote CRUD | Fixed in spec-132 for page create/delete, invites, shares and version restore. | Add per-operation loading state and cancellation. |
| CRDT coverage for structure/properties/databases | Fixed in spec-134/spec-135: server maps plus desktop seeding, projection and update planning. | Add title/CRDT and schema conflict handling when those become collaborative. |
| Awareness queue while offline | Fixed in spec-122: a one-entry pending cursor is flushed on reconnect. | Add a fixed-port reconnect integration test if the harness becomes stable. |
| Multi-line remote caret overlay | Fixed in spec-123: carets and selections use the laid-out galley. | Add bidi affinity tests if the target scripts require them. |
| Per-block persistence | Fixed in spec-133: blocks live in `workspace_blocks` and single-block edits write one row. | Add an operation log only if block-level history is required. |
| PostgreSQL / Redis | Fixed in spec-137 as optional adapters; realtime fan-out and connection pooling remain per process. | Add a shared realtime bus and pool when multi-instance editing is required. |
| Load and contention tests | `tests/perf.rs` covers workspace size, spec-125 covers mixed invariants, spec-126 adds an opt-in 8-thread read/write harness and spec-129 covers concurrent SQLite connections. | Add multi-process crash recovery tests before multi-node work. |
| Webhook I/O under the workspace lock | Fixed in spec-117/spec-155: events queue, the worker sends outside the lock on the blocking pool. | Keep the worker single and add bounded concurrency/idempotency keys for at-least-once delivery. |
| Blocking CRDT on Tokio workers | Fixed in spec-155: WebSocket sync/update run on `spawn_blocking`. | Add a per-workspace writer actor if write contention grows. |
| Awareness fan-out | Each cursor update broadcasts the full participant list (O(N) build, O(N) delivery). | Publish `AwarenessUpdated` deltas and coalesce cursor events. |
| Deep snapshot publication | HTTP writes publish an `Arc` snapshot but still clone the workspace/collaboration first (spec-151). | Move to page/block `Arc` copy-on-write; measure write p95 by workspace size first. |
| Performance regression gate | `tests/perf.rs` is opt-in and asserts correctness, not latency. | Add a scheduled (not per-PR) benchmark job with recorded artifacts. |
| Large server/UI files | `server.rs`, `sidebar.rs` and `icon_catalog.rs` still carry several responsibilities. | Split by responsibility only when the next feature touches them; see `docs/refactoring-roadmap.md`. |
| Query-level database pagination | Fixed in spec-118: index selection plus `PageWindow` materializes only the requested slice. | Add keyset cursors on property values if cursor stability becomes a requirement. |
| Renderer height tree | Fixed in spec-119: a Fenwick tree renders only the visible window. | Add a variable-width cache if resizing becomes a measured bottleneck. |
| Three HTTP layers | Fixed in spec-120/spec-121: one auth seam, a bounded parser helper, and `serve_on` now uses the Axum router. | Remove `serve_once` if its parser tests move to the Axum contract suite. |
| Emoji picker first-open scroll | Fixed in spec-156: font-stack crates are optimized in dev/test and the catalog warms in 48-glyph batches; the background renders normally again. | Measure a release `--screenshot` run after adding new emoji entries. |
| Typing pipeline | Fixed in spec-141: transaction-only CRDT updates, no per-key server persist, structural sync only on structural commands, grouped undo snapshots. | Measure p50/p95 of `CrdtSession::local_text` and `commit_edits` at 1 KB/20 KB/100 KB texts. |
| Background render behind overlays | Fixed in spec-141 for the icon picker (backdrop only). | Apply the same overlay rule to large dialogs if profiling shows it. |
| Workspace projection clone | Fixed in spec-141: the projection clones only authorized pages/resources. | Move to borrowed DTOs if allocation profiling shows it matters. |
| Snapshot cache invalidation | Fixed in spec-141: cache keyed by `workspace.version`; writes no longer clear it. | Replace the under-lock clone with an immutable `Arc` snapshot on write. |
| Request actor identity | Fixed in spec-142: member id from the session reaches writes, versions, comments and blocks. | Add the actor to webhook aggregates where they are not already covered. |
| Single-writer workspace mutex | v1 and the heavy v2 GETs read an Arc snapshot outside the lock (spec-143, spec-150); HTTP writes publish it (spec-151); writes remain serialized. | Writes still clone the workspace when publishing; measure write p95 before adding Arc-swapped state. |
| Block lookup index | Fixed in spec-144: derived map with verified fallback; refreshed on page changes, structural commands, server appends and CRDT projection. | Add an orphan/consistency check to the model test suite. |
| Session cache | Fixed in spec-145: 30 s TTL, 1024 entries, logout invalidation. | Move to a shared cache when multiple server instances must revoke across processes. |
| Patch-based undo | Fixed in spec-146: text-only frames record a page patch; structural frames keep a snapshot; undo/redo move the entry. | Move to per-block command patches if page-level patches show up in profiling. |
| Sidebar virtualization | Fixed in spec-147: flat rows painted only inside the clip rect; scroll range preserved. | Move to `show_rows` if the outer scroll area is split in a future redesign. |
| WebSocket query token | Fixed in spec-148: single-use 30 s ticket; `?token=` is rejected. | Keep proxies from logging `?ticket=` (it is single-use and short-lived). |
| Release checksums | Fixed in spec-149: a final job downloads both platforms and publishes `SHA256SUMS`. | Add Sigstore/cosign signing when keyless OIDC is available. |

## 3. Measurement plan

Before changing the persistence model, measure:

1. workspace load time at 1k / 10k / 50k blocks;
2. serialization and SQLite commit time for the same sizes;
3. collaboration log growth and snapshot size;
4. search latency for the same corpus;
5. concurrent remote reads/writes through the Axum adapter;
6. CRDT sync and projection under concurrent edits.

Record the numbers in the spec that changes the model. Do not optimize a
snapshot cost that has not been measured.

### 3.1 Baseline (spec-097, debug build, Windows)

| Blocks | Serialize | Deserialize | Search | CRDT sync | JSON |
| ------ | --------- | ----------- | ------ | --------- | ---- |
| 1k | 27 ms | 20 ms | 0.8 ms | 1.3 ms | 0.6 MiB |
| 10k | 268 ms | 184 ms | 6.1 ms | 0.9 ms | 6.2 MiB |
| 50k | 1272 ms | 899 ms | 29.6 ms | 1.0 ms | 31.1 MiB |

Run it with
`ION_OF_NOTES_PERF=1 cargo test --no-default-features --test perf -- --nocapture`.
These are single-run debug indicators, not release budgets; they show that
whole-document serialization dominates at 50k blocks while search and CRDT sync
stay cheap.

### 3.2 Tracing spans

The UI emits structured `tracing` spans at the main frame boundaries, so a
profile can separate rendering, command dispatch, persistence and I/O:

| Span | Boundary |
| ---- | -------- |
| `ui.view_state` | build the immutable `ViewState` snapshot |
| `ui.apply_commands` | apply the collected command queue |
| `ui.commit_edits` | diff, debounce and dispatch persistence |
| `ui.render_sidebar` | sidebar tree and sections |
| `ui.render_content` | page title, blocks and inline comments |

Server-side timing comes from the `tests/perf.rs` harness and the metrics
endpoint; add a span before changing a hot path so before/after numbers share a
boundary.

## 4. Non-goals for this session

- Replacing SQLite with PostgreSQL (available as an optional adapter, spec-137).
- A full CRDT model for every workspace object.
- Multi-node rate limiting beyond the shared SQLite/Redis limiter.
- Multi-node realtime fan-out (the WebSocket hub remains per process).










