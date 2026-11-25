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
- Workspace and collaboration snapshots are serialized as JSON. SQLite commits
  both in one transaction (spec-074), but the snapshot is still whole-document.
- Yrs CRDT covers page titles, block text and structure/properties/database row
  registers (spec-134, spec-135).
- Typing sends only the transaction update and local mode persists through the
  debounced REST write; the sidebar/page are not rebuilt behind the icon picker
  (spec-141).
- The Axum adapter runs the synchronous server in `spawn_blocking` behind one
  mutex, so one request at a time reaches the workspace.
- The in-memory request log is capped; sessions are not retained in the
  runtime connection list.
- SQLite updates one page row per edit and skips the position rewrite unless
  the order changed (spec-114, spec-116).
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
| Webhook I/O under the workspace lock | Fixed in spec-117: events queue, the worker sends outside the lock and applies results afterwards. | Keep the worker single and add idempotency keys for at-least-once delivery. |
| Query-level database pagination | Fixed in spec-118: index selection plus `PageWindow` materializes only the requested slice. | Add keyset cursors on property values if cursor stability becomes a requirement. |
| Renderer height tree | Fixed in spec-119: a Fenwick tree renders only the visible window. | Add a variable-width cache if resizing becomes a measured bottleneck. |
| Three HTTP layers | Fixed in spec-120/spec-121: one auth seam, a bounded parser helper, and `serve_on` now uses the Axum router. | Remove `serve_once` if its parser tests move to the Axum contract suite. |
| Typing pipeline | Fixed in spec-141: transaction-only CRDT updates, no per-key server persist, structural sync only on structural commands, grouped undo snapshots. | Measure p50/p95 of `CrdtSession::local_text` and `commit_edits` at 1 KB/20 KB/100 KB texts. |
| Background render behind overlays | Fixed in spec-141 for the icon picker (backdrop only). | Apply the same overlay rule to large dialogs if profiling shows it. |
| Workspace projection clone | Fixed in spec-141: the projection clones only authorized pages/resources. | Move to borrowed DTOs if allocation profiling shows it matters. |
| Snapshot cache invalidation | Fixed in spec-141: cache keyed by `workspace.version`; writes no longer clear it. | Replace the under-lock clone with an immutable `Arc` snapshot on write. |
| Request actor identity | Fixed in spec-142: member id from the session reaches writes, versions, comments and blocks. | Add the actor to webhook aggregates where they are not already covered. |
| Single-writer workspace mutex | v1 reads now clone and project outside the lock (spec-143); v2 reads and all writes still take one lock. | Publish an immutable per-workspace snapshot on write for v2 reads; do not just swap to `RwLock`. |
| Block lookup index | `block()` / `page_id_for_block()` still scan pages and trees. | Add a block→page index with explicit invalidation and an orphan check. |
| Session cache | Every bearer request resolves the session in the auth store. | Add a bounded in-memory cache with expiry and logout invalidation. |
| Patch-based undo | A typing burst still snapshots the whole workspace at 0.8 s intervals. | Record per-block before/after patches and group typing into one command. |
| Sidebar virtualization | The tree still draws every expanded node. | Flatten visible nodes into a list and use `show_rows`. |
| WebSocket query token | `?token=` remains as a browser fallback. | Add a short-lived single-use connection ticket and redact the query in logs. |
| Release checksums | Tagged archives are not signed and have no `SHA256SUMS`. | Publish a checksum file and consider Sigstore in the release workflow. |

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

## 4. Non-goals for this session

- Replacing SQLite with PostgreSQL.
- A browser UI.
- A full CRDT model for every workspace object.
- Multi-node rate limiting beyond the shared SQLite limiter.

