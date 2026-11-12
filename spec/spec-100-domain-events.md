# spec-100 - Domain events for webhook-emitting writes

- Status: implemented (2026-11-12)
- Related: `spec-036`, `spec-041`, `spec-087`
- Review sources: Claude and ChatGPT (handlers spell out webhook wiring)

## Purpose and scope

Page, comment, view and file-upload use cases called `emit_webhook` directly,
repeating the event type, entity type and entity id at every site. This spec
adds one domain-event seam:

- `server/events.rs` declares `DomainEvent` for the facts the server emits:
  page created/content/lock/deleted, comment created/updated/deleted, view
  created/updated/deleted and file upload created/completed;
- `WorkspaceServer::notify(event)` maps one event to the outbound webhook
  contract;
- the page, comment, view and file use cases call `notify` instead of
  `emit_webhook`.

Out of scope:

- moving `write`, persistence or change-feed calls into the event;
- block, database, workspace, share and member events;
- an async subscriber bus.

## Acceptance criteria

P1. **Given** a page create/update/lock/delete, **when** it is applied, **then**
the same webhook type, entity and payload are queued as before.

P2. **Given** a comment create/update/delete, **when** it is applied, **then**
the same comment event is queued.

P3. **Given** a view or file-upload change, **when** it is applied, **then** the
same event is queued.

P4. **Given** the webhook suite, **when** the seam is used, **then** signatures,
aggregation, retries and dead letters are unchanged.

## Test scenarios

- `tests/webhooks.rs`
- `tests/comments.rs`
- `tests/views.rs`
- `tests/blocks.rs`
- `tests/contract.rs`
- Full quality gate

## Design

- One enum and one `notify` method; no trait and no async bus.
- The mapping from domain fact to `WebhookEventType` lives in one place.
- Payloads stay byte-for-byte equivalent to the previous `json!` calls.

## Plan / tasks

- [x] Add `DomainEvent` and `notify`
- [x] Convert pages, comments, views and file uploads
- [x] Run the full gate and commit
