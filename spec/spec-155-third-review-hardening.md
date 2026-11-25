# spec-155 - Third-review security and runtime hardening

- Status: implemented (2026-11-25)
- Related: `spec-138`, `spec-145`, `spec-148`, `spec-152`, `docs/security-hardening.md`
- Review source: 2026-11-25 Kimi and ChatGPT third review

## Purpose and scope

The third review accepted the previous hardening and found a shorter list:
invite registration without proof of email control, the production proxy not
trusted, upload storage growth, plaintext remote transports, late WebSocket
limits, blocking work on Tokio workers, deep snapshot publication, and a few
browser-client defects.

### Invite tokens (S1)

- `POST /api/v2/members/{id}/invite` (owner-only) issues a random `inv_` token,
  stores only its SHA-256 hash in `CollabState.invites` and returns the raw token
  once with a 7-day expiry.
- `POST /api/v2/auth/register` now requires `invite_token`; it must match the
  invited email and be unexpired.
- The token is consumed after the account is created; a second registration
  returns 409 because the account exists.
- Issuing a new invite replaces the previous one for that member.

### Production proxy (S2)

- `compose.prod.yml` gives Caddy a fixed address (`172.30.0.2`) on an internal
  network and sets `ION_OF_NOTES_TRUSTED_PROXIES` to it by default.
- A deployment test asserts the address, subnet and environment variable.

### Remote transport TLS (S4)

- `remote::validate_remote_url` accepts `https`/`wss` everywhere and `http`/`ws`
  only for loopback (`localhost`, `127.0.0.1`, `::1`, `0.0.0.0`).
- Login, registration, `build_remote_desktop` and the desktop connect flow
  validate before sending credentials or persisting a session.

### Uploads (S3, Q1)

- Per-file limit (2 MiB), per-workspace quota (1 GiB), and a 64-pending-upload
  cap. `store_upload_content` enforces both limits; the handler returns 400.
- `prune_uploads` removes expired pending uploads and unreferenced files, called
  before creating uploads.
- The HTTP adapter now keeps `body_bytes` alongside the lossy `body`; `send`
  reads raw bytes, so binary uploads are byte-exact.
- `/metrics` exposes `ion_of_notes_upload_bytes` and
  `ion_of_notes_upload_quota_remaining`.

### Runtime (Kimi / ChatGPT P1-P2)

- A shared `LockExt::lock_or_log` logs and counts every mutex recovered from
  poisoning; `/metrics` exposes `ion_of_notes_poisoned_locks_total`.
- The WebSocket upgrade declares 1 MiB message/frame limits, closes idle sockets
  after 120 s, and tickets are bound to the workspace that issued them.
- Webhook sends and CRDT sync/update now run on `spawn_blocking`, so synchronous
  DNS/HTTP or persistence does not occupy a Tokio worker.
- The browser UI debounces search (300 ms), aborts stale searches, updates the
  local block text after a save, follows page cursors and renders block trees
  recursively.

### Deliberately left

- Awareness still broadcasts the full list on each update; coalescing/deltas are
  tracked in the performance roadmap.
- Snapshot publication still clones the workspace on HTTP writes; incremental
  `Arc` copy-on-write is deferred until write p95 justifies it.
- The single workspace writer and per-process realtime hub remain.
- Large-file responsibility splits and a performance-regression CI job remain
  roadmap items.

## Acceptance criteria

P1. **Given** no invite token, **then** registration is 403.

P2. **Given** an invite token, **then** registration succeeds once and the token
cannot be reused.

P3. **Given** the production Compose file, **then** the app trusts only Caddy's
fixed address.

P4. **Given** a non-loopback `http://` server URL, **then** the remote client
refuses it before login.

P5. **Given** an upload over the per-file limit or quota, **then** it is rejected.

P6. **Given** non-UTF-8 upload bytes, **then** the stored file is byte-identical.

P7. **Given** a lock poisoned by a panic, **then** recovery is logged and counted.

P8. **Given** a ticket issued for one workspace, **then** another workspace
cannot redeem it.

## Test scenarios

- `tests/auth.rs` (invite required, single use, change/reset).
- `tests/deployment.rs` (trusted proxy/static address).
- `tests/remote.rs` URL validation.
- `tests/comments.rs` upload limits and expiry GC.
- `tests/web.rs` binary upload and browser shell.
- `tests/realtime.rs` ticket single use and socket limits.

## Plan / tasks

- [x] Add invite tokens and require them at registration.
- [x] Fix the production proxy trust configuration.
- [x] Enforce HTTPS/WSS outside loopback.
- [x] Add upload limits, GC, metrics and binary-safe bodies.
- [x] Log/count mutex poisoning.
- [x] Tighten WebSocket upgrade/ticket/idle behavior.
- [x] Move webhook/CRDT blocking work off Tokio workers.
- [x] Fix browser search/save/pagination/tree rendering.
