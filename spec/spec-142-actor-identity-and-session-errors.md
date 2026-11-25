# spec-142 - Request actor identity and session-store errors

- Status: implemented (2026-11-25)
- Related: `spec-115`, `spec-121`, `docs/authorization.md`
- Review source: 2026-11-25 Kimi/ChatGPT second audit, findings 3.1, 5.1 and
  Kimi observations 3-5

## Purpose and scope

Write operations were still recorded as the workspace owner even when a member
performed them, and a failing session store was reported as an invalid token.

### Actor context

`WorkspaceServer` keeps the authenticated member of the current request
(`set_actor` / `current_actor`). The v2 pipeline sets it after authentication and
the v1 handler sets it after `authenticate_local_or_session`. `commit_write`,
page version history, comment `created_by`/display name, block `last_edited_by`
and webhook actors use `current_actor()`. In-process calls without a request
still fall back to the owner.

### Session-store errors

`resolve_connection` returns `io::Result<Option<Connection>>`; the authentication
seam maps a store failure to `AuthError::Unavailable`. HTTP and v2 answer `503
service_unavailable` instead of `401`, so a transient database failure does not
make clients discard a valid session.

### Smaller hardening

- `to_hex` writes each byte into the output buffer instead of allocating a
  temporary `String` per byte.

Out of scope: a session cache (tracked in the performance roadmap) and
mutex-poisoning recovery.

## Acceptance criteria

P1. **Given** a member creating a page, **then** the recorded change and version
actor is that member, not the owner.

P2. **Given** a member creating a block or comment, **then** `last_edited_by` /
`created_by` is the member.

P3. **Given** a failing `find_session`, **then** the response is 503
`service_unavailable`, not 401.

P4. **Given** a valid session, **then** authentication succeeds as before.

## Test scenarios

- `tests/authorization.rs` (member projections and page access).
- `tests/comments.rs`, `tests/blocks.rs`, `tests/contract.rs`.
- `tests/web.rs` (auth status contract).

## Plan / tasks

- [x] Add the request actor context and use it in the write path.
- [x] Attribute comments, blocks and versions to the actor.
- [x] Distinguish storage failures from invalid tokens.
- [x] Replace the per-byte hex formatting.
- [x] Keep the owner fallback for in-process calls.
