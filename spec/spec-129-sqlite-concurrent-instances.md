# spec-129 - SQLite contention from concurrent instances

- Status: implemented (2026-11-24)
- Related: `spec-039`, `spec-041`, `spec-116`, `spec-126`
- Review source: deferred multi-instance SQLite contention item

## Purpose and scope

SQLite opens enabled WAL but did not set a busy timeout before the first
`journal_mode` pragma or the schema batch. Two instances opening the same file
at the same time could fail with `database is locked` during startup. The rate
limiter read-modify-write also runs in a transaction, so concurrent connections
need a bounded busy wait to preserve the count instead of surfacing a storage
error.

This spec:

- sets `busy_timeout` through `Connection::busy_timeout` before any write
  pragma;
- moves pragmas/schema into `configure_connection`, which retries
  `DatabaseBusy` for a bounded period (50 × 20 ms) over an idempotent batch;
- makes the rate-limit read-modify-write use an `IMMEDIATE` transaction, so the
  busy handler waits for the write lock instead of failing on a deferred
  read-to-write upgrade;
- keeps the existing WAL, foreign-key and rate-limit transaction behavior;
- adds a four-connection test with a limit of 100 and 200 attempts, asserting
  exactly 100 allowed and 100 denied.

Out of scope:

- a connection pool;
- multi-process crash recovery tests;
- PostgreSQL.

## Acceptance criteria

P1. **Given** four connections opening the same database concurrently, **then**
all opens succeed.

P2. **Given** 200 concurrent rate-limit checks for one key with a limit of 100,
**then** exactly 100 are allowed and 100 denied.

P3. **Given** a normal single-instance open, **then** behavior is unchanged.

## Test scenarios

- `tests/shared_limits.rs`
  (`sqlite_rate_limit_preserves_the_count_across_concurrent_connections`,
  `sqlite_rate_limits_are_shared_across_servers`,
  `sqlite_rate_limit_window_resets`).
- `tests/sqlite_store.rs`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Busy timeout | `SqliteStore::open` |
| Bounded schema retry | `store/sqlite.rs::configure_connection` |
| Contention test | `tests/shared_limits.rs` |

## Plan / tasks

- [x] Set the busy timeout before the first write pragma.
- [x] Extract and retry the idempotent schema batch on busy.
- [x] Add the concurrent-count test.
- [x] Keep the existing shared-server test green.
