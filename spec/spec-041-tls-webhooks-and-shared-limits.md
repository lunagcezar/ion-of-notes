# spec-041 — TLS webhook delivery and shared rate limiting

## Purpose and scope

Webhook delivery currently uses a hand-written loopback TCP POST and only works
for `http://127.0.0.1`/`localhost` with an explicit override. Rate limits are
in-process maps, so two server instances do not share a workspace budget. This
spec adds a real outbound webhook adapter with rustls and a SQLite-backed shared
rate-limit store, while keeping the existing ports and behavior.

In scope:

- a `WebhookSender` port and a `UreqWebhookSender` adapter with rustls;
- timeouts, no automatic redirects and non-2xx status handling;
- injectable sender for tests;
- a `RateLimitStore` port and a SQLite implementation;
- `SqliteRateLimiter` implementing the existing decision shape;
- `WorkspaceServer` wiring for a shared limiter and a custom sender;
- tests for sender injection, loopback delivery, shared budgets and restart.

Out of scope:

- an async webhook queue, dead-letter storage or delivery dashboards;
- Redis/NATS rate limiting and multi-node coordination beyond one SQLite file;
- private CA configuration and mTLS;
- changing webhook payloads, signatures or event types.

## Evidence

- `src/server.rs` — `process_webhook_deliveries`, `send_http_post`.
- `src/webhooks.rs` — URL policy, signatures and delivery records.
- `src/server/rate_limit.rs` — in-process fixed-window limiter.
- `src/store/sqlite.rs` — SQLite adapter (spec-039).
- `docs/adr-007-production-web-server.md` — Axum/Tokio server.
- ureq 3 with the `rustls` feature.

## Acceptance criteria

1. **Given** an HTTPS webhook URL, **when** an event is delivered, **then** the
   sender uses rustls and does not require OpenSSL.
2. **Given** a non-2xx response, **when** delivery completes, **then** the
   status is recorded and the delivery is retried by the existing policy.
3. **Given** a redirect response, **when** the sender receives it, **then** it
   does not follow the redirect automatically.
4. **Given** a custom `WebhookSender`, **when** it is injected, **then**
   deliveries call it instead of the network.
5. **Given** two servers sharing one SQLite database, **when** the workspace
   request budget is exhausted, **then** both return `429` with
   `public_api_space_request_rate_limit`.
6. **Given** a SQLite rate-limit store, **when** the window expires, **then**
   the next request is allowed again.
7. **Given** the existing suite, **when** it runs, **then** all tests stay green
   and the new tests pass.
8. **Given** the dependency direction, **when** the adapters are added, **then**
   `model` and `client` stay free of ureq/SQLite types.

## Test scenarios (written before implementation)

- `tests/webhooks.rs`:
  - injected sender records calls without network;
  - existing loopback delivery still works through ureq;
  - 500 then 200 retry behavior is unchanged.
- `tests/shared_limits.rs` (new):
  - two SQLite stores share a workspace budget;
  - per-connection and workspace reasons;
  - window reset allows traffic again.
- Existing `tests/limits.rs` and `tests/sqlite_store.rs` stay green.

## Design

```text
WorkspaceServer
  ├── webhook_sender: Box<dyn WebhookSender>
  │       └── UreqWebhookSender (rustls, timeout, no redirects)
  └── rate_limiter: Box<dyn RateLimitPolicy>
          ├── RateLimiter (in-process default)
          └── SqliteRateLimiter -> Arc<dyn RateLimitStore> -> SqliteStore
```

- `src/server/webhook_sender.rs` owns the sender port and ureq adapter.
- `src/store.rs` owns `RateLimitStore`/`RateLimitHit`; `SqliteStore` implements
  it with a `rate_limits` table and a transaction per check.
- `src/server/rate_limit.rs` owns `RateLimitPolicy` and `SqliteRateLimiter`.
- `build_server_sqlite` wires the SQLite store into both persistence and the
  shared limiter.
- The default server keeps the in-process limiter and ureq sender, so existing
  behavior and tests stay valid.

## Plan / tasks

- [x] Spec + failing tests (red).
- [x] Add ureq with rustls.
- [x] `WebhookSender` port, ureq adapter, server injection.
- [x] `RateLimitStore` port, SQLite table/implementation.
- [x] `RateLimitPolicy` trait and `SqliteRateLimiter`.
- [x] Wire SQLite composition and tests.
- [x] Docs and ADR-010.
- [x] Build/test/clippy/fmt/docs/screenshot, commit.

## Result

- Added `src/server/webhook_sender.rs`: `WebhookSender` port and
  `UreqWebhookSender` with rustls, a 5-second timeout, no automatic redirects
  and non-2xx statuses returned as values.
- `WorkspaceServer` now owns an injectable sender; the default is ureq and
  tests can inject a recording sender. The old hand-written TCP POST is gone.
- Added `RateLimitStore`/`RateLimitHit` to the store ports and a
  `rate_limits` table with a transactional `SqliteStore` implementation.
- Added `RateLimitPolicy` and `SqliteRateLimiter`; `WorkspaceServer` can switch
  between the in-process limiter and a shared SQLite store.
- `build_server_sqlite_at` wires the same SQLite file into persistence and the
  shared limiter.
- Tests: 2 in `tests/shared_limits.rs` and one injected-sender test in
  `tests/webhooks.rs`; the suite is 240 tests.
- Added `docs/adr-010-webhook-delivery-and-shared-limits.md`.

## Requirement mapping

| Requirement | How |
| ----------- | --- |
| NFR (webhook TLS) | ureq + rustls sender behind a port |
| NFR (shared limits) | SQLite `rate_limits` store |
| NFR (compatibility) | in-process limiter and loopback tests stay valid |
| NFR (testability) | injectable sender and store |
