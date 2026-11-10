# ADR-010 — Webhook delivery client and shared rate-limit store

- **Status:** Accepted (2026-11-09)
- **Related:** ADR-007 (Axum server), ADR-008 (sessions/SQLite),
  spec-041 (TLS webhooks and shared limits)

## Context

Webhook delivery used a hand-written loopback TCP POST, so HTTPS endpoints did
not work. Rate limits were in-process maps, so two server instances did not
share a workspace budget. The server already has a sync core, an injectable
store port and a SQLite adapter.

## Decision

1. **ureq with rustls for webhook delivery.** The webhook worker is
   synchronous and runs inside `spawn_blocking` in the Axum adapter, so a
   blocking client fits without adding an async worker or a second runtime.
   ureq is small, supports rustls, and keeps the dependency surface low.
2. **No automatic redirects.** `max_redirects(0)` prevents a redirect from
   bypassing the existing URL/SSRF policy. A 3xx is recorded as a non-2xx
   delivery failure.
3. **Statuses are values.** `http_status_as_error(false)` lets the delivery log
   record `last_status` for 4xx/5xx instead of treating them as transport
   errors.
4. **`WebhookSender` port.** The server depends on the port; tests inject a
   recording sender and production uses `UreqWebhookSender`.
5. **Shared rate limits in SQLite.** `RateLimitStore`/`RateLimitHit` live in the
   store ports; `SqliteStore` implements a transactional fixed-window counter.
   `SqliteRateLimiter` implements the existing `RateLimitPolicy` decision
   shape, so the server and tests keep one policy interface.
6. **SQLite composition.** `build_server_sqlite_at` uses the same database for
   workspace, collaboration, auth and rate limits.

## Alternatives considered

| Alternative | Why not now |
| ----------- | ----------- |
| reqwest | Excellent async client, but heavier and would push the sync webhook worker toward async or `reqwest::blocking` (another runtime). Revisit for an async delivery queue. |
| hyper directly | Too low-level for signed POST delivery; more TLS/pooling code to own. |
| Keep the hand-written TCP POST | No TLS and no redirect/timeout policy. |
| In-process rate limits only | Two nodes do not share a workspace budget. |
| Redis rate limits | Better for multi-node scale, but adds an external service; SQLite is enough for one self-hosted node. |
| Async webhook queue | Needed for high volume and dead-letter handling, but a larger spec. |

## Consequences

Positive:

- HTTPS webhook endpoints work with rustls and no OpenSSL dependency.
- Delivery timeouts and redirect policy are explicit.
- Two server instances sharing one SQLite file share the workspace budget.
- The sender and limiter are injectable and testable without network access.

Costs:

- ureq is blocking; it relies on the Axum `spawn_blocking` boundary.
- SQLite rate limiting is single-file/single-node; Redis is still future work.
- No async queue or dead-letter store yet.
- A private CA or mTLS needs additional rustls configuration.

## Follow-up

- Add an async webhook queue with dead-letter storage and delivery metrics.
- Add Redis/NATS rate limiting for multi-node deployments.
- Add configurable CA roots for self-hosted webhook endpoints.
- Consider reqwest when the delivery worker becomes async.
