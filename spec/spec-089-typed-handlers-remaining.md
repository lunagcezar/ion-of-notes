# spec-089 - Typed handlers for the remaining resource families

- Status: implemented (2026-11-12)
- Related: `spec-087`, `spec-082`
- Review sources: Claude (typed errors) and ChatGPT (thin handlers)

## Purpose and scope

`spec-087` introduced `HandlerError`/`HandlerResult` and converted the
comments family. This spec converts the remaining resource adapters that are
straightforward, so the dispatcher attaches the request ID once and handlers
use `?`.

Converted:

- `user_file.rs` (users and file uploads);
- `blocks.rs`;
- `databases.rs`;
- `views.rs`;
- `webhooks.rs`;
- `auth.rs`.

Still legacy (small and next slice):

- `crdt.rs` (multiline `Some(match ...)` wrappers);
- `lists.rs` and `shared.rs` (shared page/workspace/list adapters).

## Acceptance criteria

P1. **Given** a converted handler, **when** it returns an error, **then** the
dispatcher attaches the request ID once through `handler_error`.

P2. **Given** a converted handler, **when** it parses a body, **then** it uses
the `body()` helper and `?`.

P3. **Given** the v2 routes, **when** they are called, **then** status codes,
envelopes and request IDs are unchanged.

P4. **Given** the existing contract, blocks, databases, views, webhooks and
auth tests, **when** the conversion is applied, **then** they pass unchanged.

## Test scenarios

- `tests/contract.rs`
- `tests/comments.rs`
- `tests/blocks.rs`
- `tests/properties.rs`
- `tests/views.rs`
- `tests/webhooks.rs`
- `tests/auth.rs`
- Full quality gate

## Design

- `fail(response)` wraps an existing `ApiResponse` as `HandlerError`.
- `ok(response)` wraps a successful `ApiResponse` as `Ok(Some(...))`.
- The v2 dispatcher maps the error with `handler_error(error, request_id)`.
- No route, DTO or response shape changes.

## Plan / tasks

- [x] Convert the six resource adapters
- [x] Update the dispatcher
- [x] Run the resource tests and full gate
- [x] Document the remaining three adapters
