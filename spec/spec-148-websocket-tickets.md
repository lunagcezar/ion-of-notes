# spec-148 - Single-use WebSocket tickets

- Status: implemented (2026-11-25)
- Related: `spec-040`, `spec-121`, `docs/security-hardening.md`
- Review source: 2026-11-25 Kimi review, observation 3 and ChatGPT item 4.7

## Purpose and scope

`/ws/workspace/{id}?token=<session>` put the session token in URLs, proxy logs,
traces and browser history. Browsers cannot set an `Authorization` header on a
WebSocket handshake, so the fallback is a short-lived, single-use ticket.

### Ticket

- `POST /api/v2/realtime/ticket` (Bearer session, read capability) returns
  `{ ticket, expires_in: 30 }`.
- `TicketStore` keeps at most 512 entries for 30 seconds.
- `redeem_realtime_ticket` removes the entry before returning it, so a ticket
  works once.
- The WebSocket handshake accepts an `authorization: Bearer` header first, then
  `?ticket=`. The old `?token=` query parameter is rejected.
- The realtime contract string now documents `?ticket=`.

The desktop remote client already sends the header, so it needs no change.

## Acceptance criteria

P1. **Given** a session, **then** `POST /api/v2/realtime/ticket` returns a
ticket and its TTL.

P2. **Given** a valid ticket, **then** the WebSocket handshake succeeds once.

P3. **Given** the same ticket again, **then** the handshake is rejected with 401.

P4. **Given** `?token=<session>`, **then** the handshake is rejected with 401.

P5. **Given** a Bearer header, **then** the handshake succeeds as before.

## Test scenarios

- `tests/realtime.rs`
  (`websocket_ticket_is_single_use_and_query_tokens_are_rejected`, plus the
  header-based ready/backlog/presence suites).
- `tests/crdt.rs` WebSocket sync suite.
- `tests/remote.rs` remote realtime suite.

## Plan / tasks

- [x] Add the bounded single-use ticket store.
- [x] Add the ticket route and policy entry.
- [x] Accept header or ticket; reject the token query parameter.
- [x] Move the tests to the Bearer header and add the ticket regression.
