# spec-122 - Offline awareness queue

- Status: implemented (2026-11-24)
- Related: `spec-040`, `spec-043`, `spec-044`, `spec-107`
- Review source: deferred item in `docs/review-response.md`

## Purpose and scope

When the remote WebSocket disconnected, awareness updates were dropped: the
bounded command channel filled and `try_send` discarded the newest cursor. On
reconnect the client did not publish the last known cursor.

This spec adds a one-entry pending-awareness slot:

- `remote::PendingAwareness` stores only the latest `Awareness` value.
- `publish_awareness` records the cursor first, then tries to wake the socket
  loop. A full or closed channel is not an error; the slot survives.
- The socket loop flushes the pending cursor immediately after a successful
  connection and whenever it receives a wake command.
- `clear_if` only clears the slot when it still holds the value that was sent,
  so a newer cursor recorded during the send is not lost.

Out of scope:

- queueing multiple historical cursors (only the latest matters);
- awareness for pages the client is not viewing;
- changing the presence protocol.

## Acceptance criteria

P1. **Given** a disconnected socket, **when** the user moves the caret,
**then** the latest cursor is retained instead of dropped.

P2. **Given** a reconnected socket, **when** the worker connects, **then** it
sends the retained cursor before reading new wake commands.

P3. **Given** a cursor recorded while a send is in flight, **when** the send
completes, **then** the newer cursor remains pending.

P4. **Given** many offline cursor moves, **then** memory stays bounded to one
entry.

## Test scenarios

- `tests/remote.rs` (`pending_awareness_keeps_only_the_latest_cursor`,
  `remote_client_receives_changes_and_awareness`).
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| One-entry slot | `remote/awareness.rs` |
| Wake command | `remote/mod.rs` `WsCommand::Awareness` |
| Flush on connect/command | `remote/realtime.rs::flush_awareness` |

## Plan / tasks

- [x] Add the bounded pending-awareness slot.
- [x] Record before waking the socket loop.
- [x] Flush on connect and on wake.
- [x] Clear only the exact value that was sent.
- [x] Add the slot regression test.
