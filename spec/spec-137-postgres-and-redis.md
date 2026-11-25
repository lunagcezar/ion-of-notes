# spec-137 - Optional PostgreSQL and Redis adapters

- Status: implemented (2026-11-24)
- Related: `spec-039`, `spec-041`, `spec-129`, `docs/adr-016-postgres-and-redis.md`
- Review source: deferred PostgreSQL/Redis item

## Purpose and scope

SQLite is the zero-service default but is single-node. This spec adds optional
multi-instance adapters without making the desktop/default build heavier.

### PostgreSQL (`--features postgres`)

`store::PostgresStore` implements:

- `WorkspaceStore`, `CollabStore` and `SnapshotStore` with the same schema
  posture as SQLite: metadata JSON, page rows without blocks, one
  `workspace_blocks` row per block, and collaboration JSON;
- `AuthStore`: accounts, sessions and pruning;
- atomic page/block/snapshot writes in one transaction.

Selection: set `ION_OF_NOTES_POSTGRES_URL`; `app::serve` uses PostgreSQL before
the `--database`/JSON branches. When the database is empty it seeds from the
JSON documents and the configured owner password.

### Redis (`--features redis`)

`store::RedisRateLimitStore` implements `RateLimitStore` with a fixed-window
counter per key. A Lua script performs `INCR` + first-hit `EXPIRE` atomically.
Selection: set `ION_OF_NOTES_REDIS_URL`; the server swaps its shared rate
limiter on any backend.

### Build and tests

- `postgres = ["dep:postgres"]`, `redis = ["dep:redis"]`,
  `external = ["postgres","redis"]`;
- default builds do not compile the clients;
- opt-in integration tests read `ION_OF_NOTES_POSTGRES_TEST_URL` and
  `ION_OF_NOTES_REDIS_TEST_URL` and no-op when unset.

Out of scope:

- schema migration tooling (tables are created idempotently);
- connection pools (one client behind a mutex per process for now);
- Redis pub/sub for realtime (the WebSocket hub remains per process);
- PostgreSQL-backed rate limits.

## Acceptance criteria

P1. **Given** the `postgres` feature and a URL, **when** the server starts,
**then** workspace, collaboration and auth state use PostgreSQL.

P2. **Given** an empty PostgreSQL database, **then** the JSON workspace is
seeded once and the owner account is created.

P3. **Given** the `redis` feature and a URL, **when** rate limits are checked,
**then** the shared budget lives in Redis.

P4. **Given** no external URL, **then** the SQLite/JSON paths are unchanged.

P5. **Given** default features, **then** neither client crate is compiled.

## Test scenarios

- `tests/postgres_store.rs` (opt-in round-trip, auth/session).
- `tests/redis_limits.rs` (opt-in fixed-window limit).
- `pwsh -File scripts/smoke-external.ps1` starts disposable containers, runs
  both opt-in tests and removes the containers.
- `cargo check --features postgres,redis`.
- `cargo check --features desktop,postgres,redis`.
- Existing SQLite/limits tests.
- Full quality gate.

## Verification evidence (2026-11-24)

Run against `postgres:16-alpine` and `redis:7-alpine` containers on ports
`15432`/`16379`:

- `tests/postgres_store.rs` and `tests/redis_limits.rs` pass with the URLs set
  (and no-op without them); they also pass with each feature enabled alone.
- `ion-of-notes-server --serve` with `ION_OF_NOTES_POSTGRES_URL` and
  `ION_OF_NOTES_REDIS_URL` served `/health` (200), `/ready` (200), `/metrics`
  (200) and `/api/v2/workspace` after login.
- The containers held the state: 13 pages / 40 block rows after seeding, one
  account and one session; a page created through the API appeared in
  `workspace_pages` and survived a server restart.
- Redis received the `ion:rate:conn:*` and `ion:rate:ws:*` counters.
- Stopping Redis mid-run logged `shared rate limit error, using local
  contingency` and requests stayed 200, which is the documented fail-safe
  behavior in `docs/security-hardening.md`.

## Design

| Piece | Location |
| ----- | -------- |
| PostgreSQL adapter | `store/postgres.rs` |
| Redis limiter | `store/redis_store.rs` |
| Feature flags | `Cargo.toml` |
| Startup selection | `app::serve`, `build_server_postgres`, `configure_redis` |

## Plan / tasks

- [x] Add optional dependencies and features.
- [x] Implement the PostgreSQL persistence/auth ports.
- [x] Implement the Redis rate-limit store.
- [x] Select both through environment variables.
- [x] Keep default SQLite/JSON behavior unchanged.
- [x] Add opt-in integration tests.
- [x] Record the decision in ADR-016.
