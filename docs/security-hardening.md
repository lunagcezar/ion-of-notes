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
- An unauthenticated `NoAuthStore` server refuses a non-loopback bind unless
  `ION_OF_NOTES_ALLOW_INSECURE_SERVER=1` is set explicitly. The static demo
  token is therefore not published to a network interface by accident.

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
- Unknown-email logins verify a constant dummy Argon2id hash before returning
  the same `401`, reducing user-enumeration timing differences.
- The remembered remote session is written with mode `0600` on Unix; an
  existing file is tightened with `set_permissions` before the token is
  rewritten. On Windows the file is created empty, receives an owner-only DACL
  through `SetNamedSecurityInfoW` (`remote/windows_acl.rs`), and only then gets
  the token. A DACL failure aborts the save. See ADR-015.
- `ConnectionKind` distinguishes a persisted session from the local demo owner;
  logout does not depend on a `"session-"` id prefix.

### Owner password

- Production server mode requires `ION_OF_NOTES_OWNER_PASSWORD` or
  `ION_OF_NOTES_OWNER_PASSWORD_FILE`.
- If neither is set, startup fails with a clear error.
- The development default (`change-me-now`) is only compiled into debug builds
  and is only used when `ION_OF_NOTES_ALLOW_DEFAULT_PASSWORD=1` is set. A
  release build still fails when that variable is the only source.
- The account is seeded once. Changing the environment variable later does not
  reset an existing password; there is no password-reset flow yet.

## 3. Login rate limiting

Login attempts use a dedicated limiter, separate from the API budget:

| Key | Default | Purpose |
| --- | ------- | ------- |
| Client IP | 5 per minute | Stops one host from spraying accounts. |
| Normalized email | 5 per minute | Stops one account from being brute-forced. |
| Workspace login bucket | 100 per minute | Bounds total login work per workspace. |

The workspace bucket is charged once per attempt; the IP and account counters
are charged separately in the same call.

The Axum adapter uses `X-Forwarded-For` (first value) or `X-Real-IP` **only**
when the direct TCP peer is loopback or listed in
`ION_OF_NOTES_TRUSTED_PROXIES` (comma-separated IPs, or `*`). A direct public
client therefore cannot rotate its login bucket with a forged header. Set the
variable to the proxy address when Caddy is not on loopback; leaving it unset
makes every request look like the proxy address.

The in-process maps purge expired windows at a size bound and deny fresh keys
while an attacker keeps them full. The shared SQLite limiter falls back to a
conservative local contingency if the store errors, instead of failing open.

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
   - Loopback, private, link-local, unique-local, CGNAT (`100.64.0.0/10`),
     TEST-NET-1/2/3, benchmarking (`198.18.0.0/15`), IETF protocol assignments,
     6to4 relay, IPv6 documentation, 6to4, Teredo and reserved (`240/4`)
     addresses are rejected.
   - IPv4-mapped and IPv4-compatible IPv6 literals are normalized through
     `to_ipv4` first, so `[::ffff:127.0.0.1]` is treated as `127.0.0.1`.
   - Plain HTTP is only allowed for loopback when
     `allow_insecure_webhooks(true)` is set (development and tests).
2. **Delivery-time resolution and pinning** (`UreqWebhookSender`):
   - The destination host and its explicit URL port are resolved with
     `ToSocketAddrs`.
   - Any private/loopback/link-local/unspecified/multicast address fails the
     delivery before an HTTP request is made.
   - The validated addresses are injected into a custom ureq `Resolver`, so the
     HTTP stack does not perform a second DNS lookup. This closes the
     DNS-rebinding window between the check and the connection.
   - Redirects remain disabled.
   - The development sender (`UreqWebhookSender::new_development`) permits
     loopback for local tests.

Remaining limitation: the process still needs an operator-level egress policy
if the host network must be restricted beyond the IP checks.

## 5.1 Session cache

Resolved bearer sessions are cached in memory for 30 seconds (bounded at 1024
entries, FIFO). Logout removes the entry immediately; any other revocation is
observed within the TTL. The auth store remains the source of truth and a store
failure is still answered with 503.

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
- Webhook delivery runs outside the workspace mutex; only the synchronous
  `process_webhook_deliveries` test/tooling wrapper sends inline.
- The loopback HTTP/1.1 parser/formatter remains as a bounded test helper;
  `serve_on` and `--serve` use the Axum router.

