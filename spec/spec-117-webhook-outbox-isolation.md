# spec-117 - Webhook outbox outside the workspace lock

- Status: implemented (2026-11-24)
- Related: `spec-036`, `spec-041`, `spec-054`, `spec-116`
- Review source: Kimi and ChatGPT performance reviews (2026-11-24)

## Purpose and scope

Webhook delivery used `ureq` while the server held the single workspace mutex.
A slow endpoint (up to five seconds) could stall unrelated reads and writes, and
a non-aggregated comment/file/view event sent synchronously inside a write.

This spec separates the persisted outbox from the transport:

1. `emit_webhook` only appends to the persisted `webhook_queue` for every event
   type. It performs no network I/O.
2. `flush_webhooks` moves queued events into persisted `WebhookDelivery`
   records with `attempts = 0` and `next_attempt_at = now`. It performs no
   network I/O.
3. The worker calls `prepare_webhook_deliveries` under the lock to collect
   `WebhookSendJob`s, releases the lock, sends each job, then reacquires the
   lock and calls `apply_webhook_delivery_results` to record status, attempts,
   backoff and dead-letter state.
4. `retry_webhook_deliveries` only resets the schedule; the worker sends.
5. `process_webhook_deliveries` remains as a synchronous convenience used by
   tests and explicit tooling. Production routes no longer call it.

Out of scope:

- concurrent senders (the worker is one task; the delivery record is the
  source of truth);
- a separate webhook process/queue;
- exactly-once delivery (at-least-once with idempotency keys is a future item).

## Acceptance criteria

P1. **Given** an active subscription, **when** an event is emitted, **then**
nothing is sent and the event is in the persisted outbox.

P2. **Given** a queued event, **when** the worker flushes, **then** a delivery
record exists but no HTTP request has been made.

P3. **Given** due deliveries, **when** the worker prepares jobs, **then** the
server lock is released before any `send`.

P4. **Given** a send result, **when** it is applied, **then** attempts, status,
error, backoff and dead-letter state are updated and persisted.

P5. **Given** the `/retry` route, **when** it is called, **then** it schedules
retries without performing network I/O.

P6. **Given** an aggregated page event, **when** several updates arrive before
a flush, **then** only the latest event per subscription/entity is delivered.

## Test scenarios

- `tests/webhooks.rs`:
  `emit_and_flush_do_not_send_until_the_worker_cycle`,
  `page_event_delivery_has_documented_fields_and_signature`,
  `comment_event_is_delivered_by_the_worker`,
  `aggregation_keeps_only_the_latest_event_per_entity`,
  `failed_delivery_is_retried`,
  `failed_delivery_is_dead_lettered_and_can_be_requeued`,
  `aggregated_events_survive_a_restart`,
  `injected_webhook_sender_receives_the_delivery`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Queue on emit | `WorkspaceServer::emit_webhook` |
| Queue to delivery records | `WorkspaceServer::flush_webhooks` |
| Due jobs | `WorkspaceServer::prepare_webhook_deliveries` |
| Apply results | `WorkspaceServer::apply_webhook_delivery_results` |
| Worker cycle | `server::web::spawn_webhook_worker` |
| Sender handle | `WebhookRuntime.sender: Arc<dyn WebhookSender>` |
| Retry scheduling | `WorkspaceServer::retry_webhook_deliveries` |

The sender is shared behind an `Arc` so the worker can clone it under the lock
and use it after releasing the lock. `UreqWebhookSender::allow_private_targets`
uses an `AtomicBool`, keeping the trait object shareable.

## Plan / tasks

- [x] Make the sender shareable and the private-target switch atomic.
- [x] Queue every webhook event; remove synchronous delivery from `emit`.
- [x] Split flush/prepare/send/apply phases.
- [x] Run the worker send outside the workspace mutex.
- [x] Make `/retry` schedule only.
- [x] Update tests and contract docs.
