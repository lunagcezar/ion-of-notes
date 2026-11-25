# spec-145 - Bounded session cache

- Status: implemented (2026-11-25)
- Related: `spec-039`, `spec-143`, `docs/security-hardening.md`
- Review source: 2026-11-25 Kimi review, observation 3

## Purpose and scope

Every bearer request resolved the opaque token against the auth store
(SQLite/PostgreSQL) even though a session rarely changes. The auth store remains
the source of truth; this spec adds a small cache in front of it.

### Design

- `SessionCache` keys on the token hash and stores `(Connection, expires_at)`.
- Entries expire after 30 seconds. A revoked session therefore stops working
  within that window even if the store is not consulted.
- The cache is bounded at 1024 entries with FIFO eviction.
- `resolve_connection` checks the cache first, then the store, then stores the
  resolved connection.
- `invalidate_session` removes an entry; logout calls it immediately.
- A session that the store reports as expired/absent is not cached.

`InvalidToken` versus `Unavailable` semantics from spec-142 are unchanged: a
store failure is still a 503.

## Acceptance criteria

P1. **Given** a valid persisted session, **then** two resolutions perform one
store lookup.

P2. **Given** `invalidate_session` (logout), **then** the next resolution reads
the store again.

P3. **Given** an expired cache entry, **then** it is dropped and the store is
consulted.

P4. **Given** more than 1024 sessions, **then** the oldest entry is evicted.

## Test scenarios

- `tests/auth.rs` (`resolved_sessions_are_cached_and_invalidated_on_logout`,
  existing login/logout/restart suites).

## Plan / tasks

- [x] Add the bounded cache with a TTL and FIFO cap.
- [x] Use it in `resolve_connection`.
- [x] Invalidate on logout.
- [x] Add the lookup-count regression test.
