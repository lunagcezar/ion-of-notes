# spec-121 - One serving path for the loopback listener

- Status: implemented (2026-11-24)
- Related: `spec-021`, `spec-038`, `spec-120`
- Review source: Kimi quality review (2026-11-24)

## Purpose and scope

`serve_on` (and therefore `serve_forever`) used the hand-written HTTP/1.1
parser and response formatter, while `--serve` used Axum. Two parsers can drift
on framing, headers and error behavior.

This spec makes `serve_on` hand its already-bound listener to the Axum adapter:

- one HTTP implementation handles v1, v2, snapshots, health, ready and metrics;
- `parse_http_request` / `format_http_response` / `serve_once` stay as a
  low-level parser and test helper, not as a second serving path;
- the request-ID header assertion accepts the case-insensitive header spelling
  Axum emits.

Out of scope:

- removing `serve_once` or the parser unit tests;
- merging the v1 and v2 adapters (both already run through the same Axum
  router).

## Acceptance criteria

P1. **Given** an already-bound `TcpListener`, **when** `serve_on` runs, **then**
requests are served by the Axum router.

P2. **Given** two concurrent loopback connections, **then** both receive
`200`.

P3. **Given** a v2 HTTP roundtrip, **then** the response carries
`x-request-id` (case-insensitive).

P4. **Given** `serve_once` in a parser test, **then** the existing
parse/format helpers still work.

## Test scenarios

- `tests/server.rs` (`serve_on_handles_two_concurrent_connections`, parser
  tests).
- `tests/contract.rs` (`v2_http_roundtrip_has_request_id_header`).
- `tests/smoke.rs`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Listener handoff | `server/http.rs::serve_on` |
| Axum router | `server/web::router_with_server` |
| Legacy parser/test helper | `server/http.rs::serve_once` |

## Plan / tasks

- [x] Move `serve_on` to a Tokio runtime + Axum router.
- [x] Keep `serve_once` and the parser tests.
- [x] Update the header assertion for case-insensitive spelling.
