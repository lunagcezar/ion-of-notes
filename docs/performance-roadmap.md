# Performance and scale roadmap

The 2026-11-12 reviews identified performance and scale items that are real
but larger than the security hardening session. This document records the
current posture, the deferred work and the measurements needed before changing
the architecture.

## 1. Current posture

- The desktop local mode is in-process and synchronous; it does not cross a
  network boundary.
- The remote desktop client calls the REST API synchronously from the UI
  thread. A slow server can block the window.
- Workspace and collaboration snapshots are serialized as JSON. SQLite commits
  both in one transaction (spec-074), but the snapshot is still whole-document.
- Yrs CRDT covers page titles and block text. Block structure, properties,
  databases, comments and shares are REST-owned.
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
| Asynchronous remote CRUD | Requires a UI request/response model and loading states; it is a product change, not a local refactor. | Add a background command queue with per-request cancellation and a busy indicator. |
| CRDT coverage for structure/properties/databases | Needs a merge model for ordered rows, property schemas and deletions. | Start with one object family (database rows) and define its conflict semantics. |
| Awareness queue while offline | The WebSocket currently drops awareness when disconnected. | Keep the latest awareness per member and flush on reconnect. |
| Multi-line remote caret overlay | The current overlay approximates first-line positions. | Project the CRDT selection through the text layout and test wrapped paragraphs. |
| Per-block persistence | Page rows are incremental (spec-114); block-level tables add schema and migration complexity. | Add a block table only if page-row size becomes a measured problem above ~20k blocks. |
| PostgreSQL / Redis | SQLite is single-node and the deployment story is intentionally small. | Keep the ports; add a PostgreSQL adapter and a shared limiter only when multi-node is a real requirement. |
| Load and contention tests | `tests/perf.rs` now covers workspace size; multi-instance SQLite contention is still unmeasured. | Add a shared-limiter contention test before multi-node work. |
| Webhook I/O under the workspace lock | Fixed in spec-117: events queue, the worker sends outside the lock and applies results afterwards. | Keep the worker single and add idempotency keys for at-least-once delivery. |
| Query-level database pagination | Fixed in spec-118: index selection plus `PageWindow` materializes only the requested slice. | Add keyset cursors on property values if cursor stability becomes a requirement. |
| Renderer height tree | Fixed in spec-119: a Fenwick tree renders only the visible window. | Add a variable-width cache if resizing becomes a measured bottleneck. |
| Three HTTP layers | Fixed in spec-120/spec-121: one auth seam, a bounded parser helper, and `serve_on` now uses the Axum router. | Remove `serve_once` if its parser tests move to the Axum contract suite. |

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
