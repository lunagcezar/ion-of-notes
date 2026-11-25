# spec-126 - Opt-in lock-contention harness

- Status: implemented (2026-11-24)
- Related: `spec-097`, `spec-111`, `spec-117`
- Review source: deferred load-test item in `docs/review-response.md`

## Purpose and scope

`tests/perf.rs` measures serialization, search and CRDT work on one thread. It
does not show how concurrent API work behaves behind the single
`WorkspaceServer` mutex.

This spec adds `tests/load.rs`:

- gated by `ION_OF_NOTES_LOAD=1`; otherwise the test returns immediately;
- builds a 1,000-page / 10,000-block workspace;
- runs 8 threads × 100 requests, alternating `GET /api/pages` and
  `PATCH /api/pages/load-0`;
- records each request duration and prints p50, p95 and max;
- asserts every request returned 2xx (it is a measurement harness, not a
  latency gate).

Out of scope:

- HTTP/TLS overhead (the harness calls `RequestTransport` directly);
- multi-process SQLite contention;
- a benchmark framework dependency.

## Acceptance criteria

P1. **Given** no `ION_OF_NOTES_LOAD`, **then** the test is a fast no-op.

P2. **Given** the load flag, **when** all threads finish, **then** every request
is 2xx and a p50/p95/max line is printed.

P3. **Given** the same build, **then** the sample size is deterministic
(`THREADS × ITERATIONS`).

## Test scenarios

- `ION_OF_NOTES_LOAD=1 cargo test --no-default-features --test load -- --nocapture`.
- Default `cargo test --all-targets --no-default-features`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Harness | `tests/load.rs` |
| Sample reporting | `println!` with p50/p95/max |

## Plan / tasks

- [x] Build a deterministic large workspace.
- [x] Run mixed reads/writes across threads.
- [x] Record and print latency percentiles.
- [x] Keep the default gate a no-op.
