# spec-140 - Awareness identity, SSRF ranges and WebSocket budget

- Status: implemented (2026-11-25)
- Related: `spec-040`, `spec-041`, `spec-036`, `docs/security-hardening.md`
- Review source: 2026-11-25 Kimi/ChatGPT second audit, findings 2.11-2.12, 4.6

## Purpose and scope

Awareness payloads were trusted for `member_id` and `name`, the SSRF
classification missed IPv4-mapped IPv6 and several special ranges, and the
WebSocket route bypassed the HTTP rate limiter.

### Awareness

- The server overwrites `session_id`, `member_id` and `name` from the
  authenticated session; client-provided identity is ignored.
- Input is accepted only when the page exists, the member may read it, the block
  belongs to that page and `index` / `selection_end` are within the block text.
- Broadcast awareness is filtered entry by entry. One unreadable cursor no
  longer drops the whole batch and no entry leaks an unreadable page.

### SSRF classification

`is_private_ip` normalizes IPv4-mapped/compatible IPv6 through `to_ipv4`, then
applies the IPv4 rules. It now also rejects RFC 6598 (`100.64.0.0/10`), IETF
protocol assignments, TEST-NET-1/2/3, benchmarking (`198.18.0.0/15`), the 6to4
relay range, IPv6 documentation (`2001:db8::/32`), 6to4, Teredo and the reserved
`240/4` block.

### WebSocket message budget

`MessageBudget` is a fixed-window bucket (`120` messages per `10 s`). A socket
that exceeds it receives `rate_limited` and is closed, so CRDT parsing, merges
and the workspace lock are bounded per connection.

## Acceptance criteria

P1. **Given** an awareness payload that claims another member, **then** the
broadcast carries the authenticated member id and name.

P2. **Given** an awareness block that belongs to another page, or an index past
the text, **then** the update is rejected with `awareness_rejected`.

P3. **Given** `https://[::ffff:127.0.0.1]/hook`, **then** the webhook target is
rejected like `127.0.0.1`.

P4. **Given** `100.64.0.1`, `198.18.0.1` or `203.0.113.9`, **then** the target
is rejected.

P5. **Given** more than 120 messages in 10 seconds on one socket, **then** the
server answers `rate_limited` and closes.

## Test scenarios

- `tests/remote.rs` (`remote_client_receives_changes_and_awareness` asserts the
  normalized identity).
- `tests/webhooks.rs` (`private_https_targets_are_rejected` includes mapped IPv6,
  CGNAT, benchmarking and TEST-NET).
- `tests/realtime.rs` (`websocket_message_budget_refills_after_the_window`).
- `tests/realtime.rs` WebSocket suites for ready/backlog/presence.

## Plan / tasks

- [x] Derive awareness identity from the session and validate its content.
- [x] Filter awareness entries individually.
- [x] Normalize IPv4-mapped IPv6 and block the special ranges.
- [x] Add a per-connection WebSocket message budget.
- [x] Add regression tests.
