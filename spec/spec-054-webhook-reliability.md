# spec-054 - Webhook reliability

- Status: implemented (2026-11-11)
- Related: `spec-036`, `spec-041`, `spec-052`, `docs/api-contract.md`

## Purpose and scope

Webhook delivery currently keeps aggregated events in an in-memory queue. A
server restart loses pending events, retries only run when an operator calls
the retry endpoint, and exhausted deliveries are not distinguishable from
ordinary failures. This spec makes the outbox persistent, runs the delivery
worker automatically, and adds a dead-letter boundary with a requeue path.

In scope:

- Persist aggregated webhook events in `CollabState` (`webhook_queue`), so a
  restart can flush them.
- Run a background webhook worker from the Axum adapter: flush aggregated
  events and retry due deliveries every `ION_OF_NOTES_WEBHOOK_INTERVAL_SECS`
  seconds (default 30).
- Respect `next_attempt_at` backoff in the worker; keep the manual `/retry`
  route as an immediate retry.
- Add `WebhookDelivery::dead_lettered`, a dead-letter list endpoint and a
  requeue endpoint.
- Expose webhook pending/dead-letter/delivery counts in `/metrics`.
- Add tests for restart persistence, dead-lettering, requeue and metrics.
- Update the API contract, architecture, deployment guide, requirements,
  glossary, spec index and changelog.

Out of scope:

- No external queue (Redis/SQS) or multi-node worker coordination.
- No automatic pruning policy for old delivery records.
- No change to the event payloads or signature format.

## Evidence

- `WebhookRuntime` keeps `queue: Vec<WebhookEvent>` in memory; `flush_webhooks`
  is only called by tests.
- `process_webhook_deliveries` retries immediately and has no dead-letter flag.
- `CollabState` already persists subscriptions and delivery records, so a
  `webhook_queue` field with `#[serde(default)]` is backward-compatible.
- `serve_axum` already owns the Tokio runtime and can spawn a periodic worker.
- `tests/webhooks.rs` covers aggregation, signatures, retries and the v2 API.

## Acceptance criteria

1. **Given** an aggregated event, **when** the server restarts before the
   worker flushes it, **then** the event is still pending and is delivered
   after the restart.
2. **Given** the Axum adapter, **when** the worker interval elapses, **then**
   it flushes aggregated events and retries deliveries whose backoff has
   elapsed without blocking the Tokio reactor.
3. **Given** a delivery that fails eight times, **when** the worker or manual
   retry runs again, **then** it is marked `dead_lettered`, is not retried
   automatically, and appears in the dead-letter endpoint.
4. **Given** a dead-lettered delivery, **when**
   `POST /api/v2/webhook_deliveries/requeue` is called, **then** it is reset
   (attempts zero, not delivered, not dead-lettered) and the next retry can
   deliver it.
5. **Given** `/metrics`, **when** it is scraped, **then** it exposes
   `ion_of_notes_webhook_pending`, `ion_of_notes_webhook_dead_letters` and
   `ion_of_notes_webhook_deliveries_total`.
6. **Given** the full gate, **when** fmt, both clippy passes, core + desktop
   tests, docs and the preview screenshots run, **then** all are green.

## Test scenarios (written before implementation)

- **Red:** add `webhook_queue` to `CollabState` and the new endpoint
  assertions before the handlers; `cargo test --test webhooks` fails.
- **Restart:** create/verify a subscription, emit an aggregated page event,
  drop the JSON-backed server, reopen it, flush and assert one delivery.
- **Dead letter:** use a failing sender, retry eight times, assert
  `dead_lettered` and the dead-letter API response; requeue and assert reset.
- **Immediate retry:** the existing `failed_delivery_is_retried` test uses the
  manual retry path.
- **Metrics:** `/metrics` contains the three webhook series.
- **Full gate:** `pwsh -File scripts/quality.ps1` and the preview screenshots.

## Design

```text
CollabState.webhook_queue        persistent aggregated outbox
WebhookDelivery.dead_lettered    terminal failure marker
WorkspaceServer
  emit_webhook                   queues aggregated events and persists
  flush_webhooks                 aggregates/delivers the persistent outbox
  process_webhook_deliveries     respects next_attempt_at, marks dead letters
  retry_webhook_deliveries       manual immediate retry
  webhook_stats                  pending/dead/delivery counts
  dead_letter_deliveries         filtered view
  requeue_webhook_delivery       reset one delivery
serve_axum                       background interval worker via spawn_blocking
/api/v2/webhook_deliveries/dead_letters
/api/v2/webhook_deliveries/requeue
/metrics                         webhook gauges/counter
```

The worker uses `tokio::time::interval` and `spawn_blocking` so the
synchronous server lock and the `ureq` sender never block the reactor. The
manual retry endpoint resets `next_attempt_at` to now before processing; the
worker does not, so backoff remains meaningful.

## Plan / tasks

- [x] Write this spec and record the red step.
- [x] Persist the aggregated outbox in `CollabState`.
- [x] Add dead-letter state, requeue and stats.
- [x] Add the background worker and `/metrics` series.
- [x] Add the dead-letter/requeue v2 routes and capabilities.
- [x] Add restart, dead-letter, requeue and metrics tests.
- [x] Update docs, spec index and changelog.
- [x] Run the full gate and commit.
