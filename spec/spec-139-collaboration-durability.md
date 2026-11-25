# spec-139 - Collaboration durability and resynchronization

- Status: implemented (2026-11-25)
- Related: `spec-011`, `spec-054`, `spec-120`, `docs/persistence-integrity.md`
- Review source: 2026-11-25 Kimi/ChatGPT second audit, findings 2.13, 4.1-4.5

## Purpose and scope

The second audit found that several "durable" paths still degrade silently: a
corrupt collaboration document was replaced by a seed, a failed webhook persist
only printed a line, retention could drop a pending delivery, and a stale sync
cursor or a lagged WebSocket was not reported to the client.

### Collaboration document

`store::load_collab_or_seed` now returns `io::Result<CollabState>`. Only
`NotFound` seeds and saves a new document; every other error is returned and the
file is left byte-for-byte untouched. `WorkspaceServer::with_paths` returns
`io::Result<Self>` so startup fails closed instead of running with a fresh
collaboration state.

### Webhook outbox

- `persist_outbox` marks the outbox non-durable when persistence fails.
- `prepare_webhook_deliveries` refuses to send anything while the outbox is not
  durable and retries the persist first.
- Retention removes only delivered or dead-lettered records. If every record is
  pending, no record is removed and `enqueue_delivery` refuses new work
  (backpressure); refused events stay in the queue.
- The event queue is bounded (`MAX_WEBHOOK_QUEUE`) and logs a dropped event
  rather than growing without limit.
- Subscription lookup uses a map instead of a scan per delivery.

### Sync log

- `SyncBatch` carries `oldest_seq` and `reset_required`.
- `changes_since` sets `reset_required` when `since` predates the retained
  window.
- The WebSocket `ready` message carries the same flag and suppresses the
  backlog when a full resync is required.
- A lagged broadcast receiver sends `resync` and closes the socket; the remote
  client resets its cursor to 0 before reconnecting.
- The change log is a `VecDeque`, so trimming is O(1)-ish instead of shifting a
  `Vec`.

### Deleted pages

`change_visible` resolves access from the persisted shares rather than requiring
the page to still exist, so a delete event reaches members who had access.

## Acceptance criteria

P1. **Given** a corrupt collaboration file, **when** it is loaded, **then** an
error is returned and the bytes are unchanged.

P2. **Given** a missing collaboration file, **then** the seed is saved.

P3. **Given** a failed outbox persist, **then** no delivery is sent until a
later persist succeeds.

P4. **Given** more pending deliveries than the retention bound, **then** none
are dropped and new deliveries are refused.

P5. **Given** a cursor older than the retained log, **then** `reset_required` is
true and the WebSocket ready message says so.

P6. **Given** a lagged broadcast receiver, **then** the client receives `resync`
and the connection closes.

P7. **Given** a member shared on a page that is then deleted, **then**
`change_visible` still returns true for that page id.

## Test scenarios

- `tests/collab.rs` (`a_cursor_older_than_the_retained_log_requires_a_full_resync`).
- `tests/collab_store.rs` (`corrupt_collab_file_is_preserved_and_reported`).
- `tests/moves.rs` (`deleted_page_changes_stay_visible_to_a_shared_member`).
- `tests/webhooks.rs` (outbox persistence, dead-letter and requeue suites).
- `tests/realtime.rs` (ready message and backlog).

## Plan / tasks

- [x] Make collaboration loading non-destructive.
- [x] Gate webhook sends on a durable outbox and stop dropping pending records.
- [x] Add the reset signal to the REST and WebSocket sync paths.
- [x] Report lagged WebSocket receivers and reset the client cursor.
- [x] Keep delete events visible through the share rules.
- [x] Trim the change log with a deque.
