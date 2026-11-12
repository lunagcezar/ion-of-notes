# Security hardening

This document records the authentication and outbound-request hardening added
in response to the 2026-11-12 external reviews. It is the operator and
contributor reference for the server's security boundary.

## 1. Authentication boundary

| Mode | Auth store | v1 API | v2 API | Demo owner token |
| ---- | ---------- | ------ | ------ | ---------------- |
| Local desktop | `NoAuthStore` | Public (in-process) | Requires the local owner token | Registered |
| Self-hosted JSON | `NoAuthStore` | Public (local process) | Requires the local owner token | Registered |
| SQLite server | `SqliteStore` | Requires a session bearer token | Requires a session bearer token | **Not registered** |
| Tests / custom | `MemoryAuthStore` or another real store | Requires a session bearer token | Requires a session bearer token | **Not registered** |

- `/health` is public in every mode.
- `/ready` and `/metrics` are Axum operational routes and stay public; keep the
  listener behind the reverse proxy and do not publish the application port.
- `AuthStore::requires_auth()` is the single switch. Only `NoAuthStore`
  returns `false`; every production-capable store defaults to `true`.
- `WorkspaceServer::with_stores_and_auth` registers the demo owner connection
  only when `requires_auth()` is `false`. A real auth store cannot be bypassed
  with `Authorization: Bearer owner-token`.
- The remote desktop client attaches its session token to v1 as well as v2
  requests, so remote mode does not depend on the legacy public path.

### Deployment rule

The Caddy production stack must keep the application port internal. Caddy
terminates TLS and forwards the request; the application still enforces the
session token. The reverse proxy is not an authentication layer.

## 2. Sessions and passwords

- Passwords use Argon2id PHC strings; raw passwords are never stored.
- Session tokens are opaque, generated from OS randomness, and only their
  SHA-256 hash is stored.
- Sessions expire (`SESSION_DAYS`); expired sessions are deleted on lookup.
- Logout deletes the session. Sessions are resolved from the auth store on each
  request, so the runtime does not retain one connection object per login.
- Unknown-email logins verify a dummy Argon2id hash before returning the same
  `401`, reducing user-enumeration timing differences.
- The remembered remote session is written with mode `0600` on Unix before any
  token bytes are written. On Windows the file relies on the user profile's
  ACL; use a dedicated OS account for the desktop app.

### Owner password

- Production server mode requires `ION_OF_NOTES_OWNER_PASSWORD` or
  `ION_OF_NOTES_OWNER_PASSWORD_FILE`.
- If neither is set, startup fails with a clear error.
- The development default (`change-me-now`) is only used when
  `ION_OF_NOTES_ALLOW_DEFAULT_PASSWORD=1` is set explicitly.
- The account is seeded once. Changing the environment variable later does not
  reset an existing password; there is no password-reset flow yet.

## 3. Login rate limiting

Login attempts use a dedicated limiter, separate from the API budget:

| Key | Default | Purpose |
| --- | ------- | ------- |
| Client IP | 5 per minute | Stops one host from spraying accounts. |
| Normalized email | 5 per minute | Stops one account from being brute-forced. |
| Workspace login bucket | 100 per minute | Bounds total login work per workspace. |

The Axum adapter fills `ApiRequest::client_ip` from `X-Forwarded-For` (first
value) or `X-Real-IP`. The production proxy must set or strip these headers;
do not trust client-supplied values on a directly exposed listener.

## 4. Resource limits and timeouts

- The in-memory v2 request log is capped (`DEFAULT_REQUEST_LOG_CAP`, 1000
  entries); older entries are dropped.
- The Axum adapter applies a 30-second request timeout
  (`REQUEST_TIMEOUT_SECS`) on top of the existing body-size limit.
- Session connections are not retained in `runtime.connections`, so repeated
  logins do not grow the connection list.
- The API rate limiter remains the documented per-connection and
  per-workspace budget; a shared SQLite limiter is available for multi-instance
  deployments.

## 5. Webhook egress

Webhook URLs are validated in two layers:

1. **Creation-time policy** (`webhooks::is_allowed_url`):
   - HTTPS is accepted only for public hosts.
   - Loopback, private, link-local, unique-local and multicast addresses are
     rejected.
   - Plain HTTP is only allowed for loopback when
     `allow_insecure_webhooks(true)` is set (development and tests).
2. **Delivery-time resolution** (`UreqWebhookSender`):
   - The destination host is resolved with `ToSocketAddrs`.
   - Any private/loopback/link-local/unspecified/multicast address fails the
     delivery before an HTTP request is made.
   - Redirects remain disabled.
   - The development sender (`UreqWebhookSender::new_development`) permits
     loopback for local tests.

Remaining limitation: DNS rebinding can still change the address between the
resolution check and the connection. An operator-level egress allowlist or
network policy is the durable mitigation.

## 6. Operational checklist

- [ ] Set a strong owner password or mount a secret file.
- [ ] Keep the application port internal; expose only Caddy.
- [ ] Set or strip `X-Forwarded-For`/`X-Real-IP` at the proxy.
- [ ] Keep the SQLite database and `remote-session.json` on a private volume.
- [ ] Run `cargo audit` in CI and review Renovate updates for auth/HTTP
      dependencies.
- [ ] Back up the database and test a restore (see
      `docs/persistence-integrity.md`).
- [ ] Treat this build as an early-stage project until an independent security
      review covers auth, sessions and webhooks.

## 7. Known limitations

- No registration, password reset or account-management UI yet.
- No external security audit.
- Nested-resource authorization is partial; see `docs/authorization.md`.
- Windows session-file ACLs are inherited from the profile rather than set
  explicitly.
- The webhook DNS check is not a full DNS-rebinding defense.
