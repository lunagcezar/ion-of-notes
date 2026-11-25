# spec-120 - HTTP adapter authentication seam and parser hardening

- Status: implemented (2026-11-24)
- Related: `spec-021`, `spec-038`, `spec-073`, `spec-116`
- Review source: Kimi quality review (2026-11-24)

## Purpose and scope

The v1 adapter, the Axum snapshot helper and the v2 pipeline each turned a
Bearer header into a `Connection` with slightly different code. The loopback
HTTP/1.1 parser also accepted ambiguous `Content-Length` values, truncated
bodies, non-origin paths and an unbounded head, and treated a malformed request
as `/health`.

This spec:

- adds `server::authentication` with `AuthError`,
  `authenticate_bearer` and `authenticate_local_or_session`;
- uses the same seam in v1, the Axum snapshot path and v2 (v2 maps the error to
  its structured response);
- hardens `parse_http_request`:
  - 16 KiB head bound and 100-header bound;
  - origin-form path only;
  - strict numeric `Content-Length`, rejecting duplicate conflicting values;
  - rejecting a body shorter than `Content-Length`;
  - no fallback request when parsing fails;
- makes `serve_once` and `serve_on` return a `400` envelope for a malformed
  request instead of pretending it was `/health`.

Out of scope (documented as a separate refactor):

- retiring the loopback parser in favour of Axum for `serve_on`;
- removing the v1 adapter.

## Acceptance criteria

P1. **Given** a valid token, **when** v1, v2 or the snapshot path runs, **then**
all three reach the same `Connection` resolution.

P2. **Given** a production auth store, **when** v1 receives no token, **then**
the shared seam returns `MissingToken` and v1 maps it to `401`.

P3. **Given** `NoAuthStore`, **when** v1 receives no token, **then** the local
owner connection is returned.

P4. **Given** a non-numeric or duplicate conflicting `Content-Length`, **then**
the parser returns `None`.

P5. **Given** a body shorter than its declared length, **then** the parser
returns `None`.

P6. **Given** a malformed request on the loopback listener, **then** the client
receives `400`, not `/health`.

## Test scenarios

- `tests/server.rs`:
  `parse_http_request_rejects_ambiguous_framing_and_truncated_bodies`,
  `parse_http_request_rejects_an_oversized_head`,
  existing `parse_http_request_extracts_method_path_body`.
- `tests/web.rs` (`v2_requires_version_then_token`,
  `snapshot_reads_require_auth_when_configured`).
- `tests/security.rs` (`v1_requires_a_token_when_auth_is_configured`,
  `v1_stays_public_for_the_local_no_auth_store`).
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Auth seam | `server/authentication.rs` |
| v1 call | `server/routes/v1.rs` |
| v2 call | `server/routes/pipeline.rs` |
| Snapshot call | `server/web/http.rs` |
| Parser | `server/http.rs` |

## Plan / tasks

- [x] Add the shared authentication seam.
- [x] Use it in v1, v2 and the snapshot path.
- [x] Bound the parser head and header count.
- [x] Reject ambiguous/truncated framing.
- [x] Return `400` for malformed loopback requests.
- [x] Add parser regression tests.
