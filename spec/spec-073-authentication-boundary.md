# spec-073 - Authentication boundary and security hardening

- Status: in progress (2026-11-12)
- Related: `spec-030`, `spec-039`, `spec-041`, `spec-045`, `spec-056`
- Review sources: the 2026-11-12 external reviews (Kimi, ChatGPT, Claude)

## Purpose and scope

The external reviews found that the production-capable server still exposes a
legacy unauthenticated v1 surface and a fixed `owner-token` demo credential,
even when a real auth store is configured. They also flagged login rate-limit
granularity, unbounded in-memory growth, session-file permissions and missing
request timeouts.

This spec makes the authentication boundary explicit and fail-closed for
production-capable servers while preserving the local desktop mode.

Scope:

- an `AuthStore::requires_auth()` capability: `NoAuthStore` is the only
  adapter that returns `false`;
- v1 API authentication when an auth store is configured (`/health` stays
  public);
- register the demo owner connection only for the local `NoAuthStore`;
- make the desktop client attach its bearer token to v1 requests in remote
  mode;
- per-client and per-account login rate limiting;
- cap the in-memory request log and stop growing the connection list on every
  login;
- write `remote-session.json` with private permissions before any token bytes
  are written (Unix) and document the Windows limitation;
- add a request timeout layer to the Axum adapter;
- fail closed when no owner password is configured, with an explicit
  development override;
- constant-time-ish login handling for unknown emails (dummy Argon2 verify).

Out of scope:

- principal-aware page authorization (spec-076);
- account invitation/password reset lifecycle (documented as a roadmap item);
- replacing the in-memory rate limiter with Redis/PostgreSQL.

## Acceptance criteria

P1. **Given** a server constructed with a real auth store, **when** a v1
request arrives without a valid bearer token, **then** it is rejected with
`401`; `/health` remains public.

P2. **Given** a server constructed with `NoAuthStore`, **when** the local
desktop sends a v1 request without a token, **then** it still succeeds.

P3. **Given** a server constructed with a real auth store, **when**
`owner-token` is used, **then** it is not a valid connection and the request is
rejected.

P4. **Given** a remote desktop client with a session token, **when** it calls a
v1 endpoint, **then** the bearer token is attached.

P5. **Given** repeated login attempts for one client or one email, **when**
the configured login budget is exhausted, **then** the next attempt returns
`429` with retry guidance.

P6. **Given** many v2 requests, **when** the request log exceeds its cap,
**then** older entries are dropped instead of growing without bound.

P7. **Given** repeated logins, **when** sessions are created, **then** the
server does not keep one connection object per session forever.

P8. **Given** a Unix profile, **when** a remote session is saved, **then** the
file is created with mode `0600` without a write-then-chmod race.

P9. **Given** the Axum adapter, **when** a request runs longer than the
configured timeout, **then** the adapter returns a timeout response.

P10. **Given** a SQLite server with no owner password configured, **when** it
starts, **then** it fails with a clear error unless
`ION_OF_NOTES_ALLOW_DEFAULT_PASSWORD=1` is set explicitly.

## Test scenarios

- `tests/security.rs::v1_requires_a_token_when_auth_is_configured`
- `tests/security.rs::v1_stays_public_for_the_local_no_auth_store`
- `tests/security.rs::owner_token_is_not_registered_for_a_real_auth_store`
- `tests/security.rs::remote_client_attaches_the_token_to_v1`
- `tests/security.rs::login_is_limited_per_client_and_per_account`
- `tests/security.rs::request_log_is_capped`
- `tests/security.rs::login_does_not_grow_the_connection_list`
- `tests/remote.rs::saved_session_is_private_on_unix` (`cfg(unix)`)
- `tests/web.rs::slow_requests_time_out` (smoke-level, optional)
- `tests/sqlite_store.rs::sqlite_server_requires_an_explicit_owner_password`

## Design

- `auth::AuthStore` gains `fn requires_auth(&self) -> bool { true }`;
  `NoAuthStore` overrides it to `false`.
- `WorkspaceServer::with_stores_and_auth` checks `requires_auth()` before
  registering `Connection::owner`.
- `handle_v1` resolves a connection from the bearer token when auth is
  required; `/health` is exempt. `response::unauthorized` returns the v1
  envelope with status 401.
- `Client::call` adds the bearer header when `self.token` is set, so remote v1
  calls are authenticated without duplicating request builders.
- Login uses a dedicated `RateLimiter` keyed by `client_ip` and by normalized
  email; `ApiRequest` gains an optional `client_ip` populated from the proxy
  headers in the Axum adapter.
- `Runtime` gains `request_log_cap` and `login_limiter`; `login` stops pushing
  session connections into `runtime.connections`.
- `remote::save_session_at` opens the file with `0600` on Unix before writing;
  Windows keeps the profile-directory permission model and is documented.
- `router_with_server` adds `tower_http::timeout::TimeoutLayer` (feature
  `timeout`).
- `app::owner_password` requires an env/file value; the development default is
  only used when `ION_OF_NOTES_ALLOW_DEFAULT_PASSWORD=1`.
- Unknown-email logins verify a dummy Argon2id hash before returning the same
  `401`, reducing timing enumeration.

## Plan / tasks

- [ ] Write spec-073 and failing security tests (red)
- [ ] Add `AuthStore::requires_auth` and the v1 auth boundary
- [ ] Remove the production owner-token path and attach tokens to v1 clients
- [ ] Add login rate limits, request-log cap and connection cleanup
- [ ] Harden session-file creation and add the request timeout
- [ ] Fail closed on the owner password
- [ ] Add documentation (`docs/security-hardening.md`) and update AGENTS/README
- [ ] Run the full gate and commit
