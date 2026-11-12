# spec-087 - Typed handler errors and comments use case

- Status: implemented (2026-11-12)
- Related: `spec-035`, `spec-082`, `spec-083`
- Review sources: Claude (typed errors, content.rs responsibilities) and
  ChatGPT (feature-oriented use cases)

## Purpose and scope

The comment routes lived inside a 645-line `content.rs` that also handled
users and file uploads, and every early return built a `v2_error` by hand.
This spec extracts comments into a use case and introduces the typed handler
error foundation.

Scope:

- `server/error.rs`: `HandlerError`, `HandlerResult<T>`, `body<T>()` and
  `handler_error`;
- `server/usecases/comments.rs`: list/create/update/delete comments, attachment
  validation, parent resolution, display-name resolution and webhook emission;
- `server/handlers/comments.rs`: the v2 comment adapter returns
  `HandlerResult<Option<ApiResponse>>` and uses `?`;
- `server/handlers/user_file.rs`: users and file uploads remain in their own
  adapter (renamed from `content.rs`);
- `usecases/comments.rs` owns `MAX_COMMENT_ATTACHMENTS = 3`.

Out of scope:

- converting every remaining handler to `HandlerResult`;
- splitting users and file uploads into separate use cases;
- actor propagation for `created_by`/webhook actors.

## Acceptance criteria

P1. **Given** a malformed comment body, **when** the route runs, **then** it
returns `400 invalid_json` through the typed `body()` helper.

P2. **Given** a comment with more than three attachments, **when** it is
created or updated, **then** the use case returns `validation_error` with the
documented limit.

P3. **Given** a comment with a page or block parent, **when** it is created,
**then** the parent is resolved and the page/block validation is unchanged.

P4. **Given** a comment create/update/delete, **when** it succeeds, **then**
the existing persistence, change feed and webhook behavior is unchanged.

P5. **Given** the existing comment and user/file tests, **when** the split is
applied, **then** they pass unchanged.

## Test scenarios

- `tests/comments.rs` (roundtrip, block parent, attachments, pagination,
  delete, users and file uploads)
- `tests/client.rs` (client comment/upload roundtrip)
- `tests/contract.rs` (v2 error shape)
- Full quality gate

## Design

- `HandlerError` is the v2 adapter error type; `handler_error` attaches the
  request ID once.
- The comments handler is a thin adapter: parse, call use case, map the
  `UseCaseError` to `HandlerError`, wrap in the existing envelope.
- The use case owns validation and mutation; it never builds a response.
- `content.rs` is renamed `user_file.rs` because comments no longer live there.

## Plan / tasks

- [x] Write spec-087
- [x] Add the typed error foundation
- [x] Extract the comments use case and handler
- [x] Rename the remaining user/file adapter
- [x] Run the full gate and commit
