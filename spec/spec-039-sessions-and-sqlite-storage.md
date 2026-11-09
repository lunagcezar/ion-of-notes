# spec-039 — Sessions, Argon2id and SQLite storage

## Purpose and scope

The server still uses the demo `owner-token` and JSON snapshots. This spec adds
the first production persistence and authentication slice: a SQLite adapter
behind the existing store ports, Argon2id password hashes, opaque sessions and
v2 login/logout/session routes. JSON stays the default for the desktop and as a
migration source; `--database` selects SQLite for `--serve`.

In scope:

- `SqliteStore` implementing `WorkspaceStore`, `CollabStore` and `AuthStore`;
- JSON → SQLite migration when the database is empty;
- `Account` and `Session` types, `AuthStore` port, in-memory and null adapters;
- Argon2id password hashing and constant-time verification;
- opaque session tokens stored as SHA-256 hashes with expiry;
- `POST /api/v2/auth/login`, `POST /api/v2/auth/logout`,
  `GET /api/v2/auth/session`;
- `--database <path>` CLI flag; JSON remains the fallback;
- tests for storage, hashing, login, logout, restart and migration.

Out of scope:

- browser cookies, CSRF, password reset, email verification and account UI;
- PostgreSQL, connection pooling and normalized page/block tables;
- WebSocket/CRDT real-time transport (spec-040);
- multi-tenant workspace creation;
- changing v1 or existing v2 payloads.

## Evidence

- ADR-003 (self-hosted server with accounts and sessions).
- ADR-007 (Axum/Tokio/Tower server).
- `spec/spec-019-collaboration-persistence.md` — the `CollabStore` port.
- `spec/spec-030-api-v2-foundation.md` — Bearer auth and capabilities.
- Current code: `src/store.rs` (ports/JSON adapters), `src/server.rs`
  (`connections`, `connection_for_token`), `src/app.rs` (composition root),
  `src/cli.rs` (`--serve`).

## Acceptance criteria

1. **Given** `--serve ... --database path.db`, **when** the server starts,
   **then** workspace and collaboration snapshots persist in SQLite.
2. **Given** an empty SQLite database and existing JSON documents, **when**
   the server starts, **then** the JSON data is migrated into SQLite and the
   JSON files remain untouched.
3. **Given** a persisted workspace, **when** the database is reopened, **then**
   the workspace and collaboration state round-trip exactly.
4. **Given** a valid email and password, **when** `POST /api/v2/auth/login` is
   called, **then** the response contains a session token, expiry and member
   fields.
5. **Given** a wrong password or unknown email, **when** login is called,
   **then** it returns `401 unauthorized` and no token.
6. **Given** a session token, **when** it is used as a Bearer token, **then**
   v2 requests succeed; after logout the same token returns `401`.
7. **Given** a session created before a server restart, **when** the SQLite
   database is reopened, **then** the token still authenticates.
8. **Given** an account, **when** it is stored, **then** the password is an
   Argon2id PHC string, never plaintext; the session table stores only a
   SHA-256 token hash.
9. **Given** no auth store (tests, desktop), **when** the demo `owner-token` is
   used, **then** the existing behavior is unchanged.
10. **Given** the dependency direction, **when** the adapters are added,
    **then** no SQLx/rusqlite/Argon2 type appears in `model`, `client` or `ui`.

## Test scenarios (written before implementation)

- `tests/auth.rs` (new):
  - Argon2id hash/verify round-trip and wrong-password rejection;
  - login success, bad password, unknown email;
  - session token authenticates v2 and logout revokes it;
  - session survives a SQLite reopen;
  - session table stores hashes, not raw tokens.
- `tests/sqlite_store.rs` (new):
  - workspace/collab round-trip;
  - account/session round-trip and pruning;
  - JSON → SQLite migration through `build_server_sqlite_at`.
- `tests/cli.rs`: `--database` parsing.
- Existing suite stays green, including the demo owner token.

## Design

```text
app::build_server_sqlite
   -> SqliteStore::open
   -> load workspace/collab or migrate JSON
   -> seed owner account (ION_OF_NOTES_OWNER_PASSWORD or dev default)
   -> WorkspaceServer::with_stores_and_auth
          -> WorkspaceStore  --+
          -> CollabStore      +--> SqliteStore (Arc<Mutex<rusqlite::Connection>>)
          -> AuthStore       --+
```

- `src/auth.rs` owns `Account`, `Session`, `LoginInput`, `SessionResponse`,
  `SessionInfo`, the `AuthStore` port, `NoAuthStore`, `MemoryAuthStore`,
  Argon2id hashing and token helpers.
- `src/store/sqlite.rs` owns `SqliteStore`; it stores workspace/collab JSON
  documents in SQLite first (pragmatic document store), with normalized tables
  left for a later spec.
- `WorkspaceServer` gains an `AuthStore`; `with_stores` keeps `NoAuthStore`
  for compatibility, `with_stores_and_auth` injects the real adapter.
- `connection_for_token` returns an owned `Connection` and falls back to the
  session table, so sessions survive restarts.
- Login is the only v2 route handled before Bearer auth; logout/session are
  handled after the connection is resolved.
- The session token is `sess_<uuid>`; the stored hash is SHA-256 hex. The
  default session lifetime is 30 days.
- The owner account is seeded once. `ION_OF_NOTES_OWNER_PASSWORD` sets the
  password; without it, a documented development default is used and a warning
  is printed.

## Plan / tasks

- [x] Spec + failing tests (red).
- [x] `auth.rs` types, hashing, token helpers and memory/null stores.
- [x] `SqliteStore` and JSON migration.
- [x] `WorkspaceServer` auth injection, login/logout/session routes.
- [x] `--database` CLI and composition root.
- [x] Tests and docs.
- [x] Build/test/clippy/fmt/docs/screenshot, commit.

## Result

- Added `src/auth.rs`: `Account`, `Session`, `LoginInput`, `SessionResponse`,
  `SessionInfo`, the `AuthStore` port, `NoAuthStore`, `MemoryAuthStore`,
  Argon2id hashing, opaque token generation and SHA-256 token hashing.
- Added `src/store/sqlite.rs`: `SqliteStore` implementing `WorkspaceStore`,
  `CollabStore` and `AuthStore`, with workspace/collab JSON documents,
  accounts and sessions in SQLite (WAL).
- Added `build_server_sqlite` / `build_server_sqlite_at`: empty SQLite
  databases migrate the existing JSON documents; the owner account is seeded
  from `ION_OF_NOTES_OWNER_PASSWORD` (development default documented).
- Added `POST /api/v2/auth/login`, `POST /api/v2/auth/logout` and
  `GET /api/v2/auth/session`; sessions survive restarts through SQLite.
- `--database <path>` selects SQLite for `--serve`; JSON remains the default
  and the desktop fallback.
- Tests: 5 in `tests/auth.rs`, 3 in `tests/sqlite_store.rs` and one CLI test;
  the suite is 231 tests.

## Requirement mapping

| Requirement | How |
| ----------- | --- |
| FR (accounts/sessions) | Argon2id + opaque sessions + v2 auth routes |
| NFR (durable storage) | SQLite behind `WorkspaceStore`/`CollabStore` |
| NFR (migration) | empty SQLite imports existing JSON |
| NFR (security) | hashed passwords and hashed session tokens |
| NFR (compatibility) | demo owner token and JSON fallback unchanged |
