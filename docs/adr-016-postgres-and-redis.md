# ADR-016 - Optional PostgreSQL and Redis adapters

- **Status:** Accepted (spec-137 implemented)
- **Date:** 2026-11-24
- **Supersedes:** none; extends the SQLite decision in ADR-008
- **Related:** `spec/spec-137-postgres-and-redis.md`,
  `docs/deployment.md`, `docs/persistence-integrity.md`

## Context

SQLite is intentionally single-node: one connection behind a process mutex,
WAL for local concurrency and a shared rate-limit table for multiple instances
on one file. For a multi-instance deployment, PostgreSQL is the natural
workspace/auth store and Redis the natural shared rate-limit backend.

The project keeps dependencies small, so making these mandatory would penalize
the desktop and small self-hosted builds.

## Decision

Add both as **optional Cargo features**:

- `postgres` enables `PostgresStore` (`WorkspaceStore`, `CollabStore`,
  `SnapshotStore`, `AuthStore`);
- `redis` enables `RedisRateLimitStore` (`RateLimitStore`);
- `external` enables both.

Selection is by environment at server startup:

- `ION_OF_NOTES_POSTGRES_URL` switches persistence/auth to PostgreSQL;
- `ION_OF_NOTES_REDIS_URL` switches the shared rate limiter to Redis on any
  backend.

Without the features, the variables are ignored and the existing JSON/SQLite
paths remain the default. Database schemas are created on open
(`CREATE TABLE IF NOT EXISTS`); there is no migration framework yet.

## Consequences

- Desktop/default builds do not compile or link either client.
- Multi-instance deployments can run the same server binary with
  `--features external`.
- Integration tests are opt-in (`ION_OF_NOTES_POSTGRES_TEST_URL`,
  `ION_OF_NOTES_REDIS_TEST_URL`) because they need external services.
- SQLite remains the zero-service default and the desktop store.
