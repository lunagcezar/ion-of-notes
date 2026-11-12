# spec-090 - Complete typed handler conversion

- Status: implemented (2026-11-12)
- Related: `spec-087`, `spec-089`
- Review sources: Claude (typed errors)

## Purpose and scope

Finish the handler conversion started in `spec-087`/`spec-089`:

- `crdt.rs` uses `body()` and typed `ok`/`fail` results;
- `lists.rs` uses typed results and `PageRequest` errors;
- `shared.rs` (workspace/pages/shares) uses typed results and a single
  `use_case_error` mapping.

After this spec every v2 resource adapter returns
`HandlerResult<Option<ApiResponse>>`; the dispatcher attaches the request ID
once.

## Acceptance criteria

P1. **Given** any v2 resource handler, **when** it returns an error, **then**
the dispatcher attaches the request ID through `handler_error`.

P2. **Given** a CRDT sync/update route, **when** the body or base64 input is
invalid, **then** it returns the same `400 validation_error` as before.

P3. **Given** a list or shared page route, **when** it succeeds or fails, **then**
the status, envelope and request ID are unchanged.

P4. **Given** the full suite, **when** the conversion is applied, **then** all
tests pass.

## Test scenarios

- `tests/contract.rs`
- `tests/crdt.rs`
- `tests/pagination.rs`
- `tests/views.rs`, `tests/properties.rs`
- `tests/authorization.rs`
- Full quality gate

## Design

- `body()` handles JSON parse errors.
- `ok()`/`fail()` wrap existing `ApiResponse` values.
- `shared.rs` maps `UseCaseError` once through `HandlerError`.
- No route or response shape changes.

## Plan / tasks

- [x] Convert `crdt.rs`, `lists.rs`, `shared.rs`
- [x] Update the dispatcher
- [x] Run the full gate and commit
