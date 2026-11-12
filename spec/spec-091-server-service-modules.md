# spec-091 - Server service modules

- Status: implemented (2026-11-12)
- Related: `spec-047`, `spec-087`, `spec-090`
- Review sources: Claude and ChatGPT (WorkspaceServer owns too many concerns)

## Purpose and scope

Auth and webhook logic lived in the generic `handlers/` tree alongside the
resource adapters. This spec moves them into a `server/services/` module so the
handler tree only contains resource families and the service seam is explicit.

Scope:

- `server/services.rs` facade;
- `server/services/auth.rs` (login, logout, session);
- `server/services/webhooks.rs` (subscriptions, deliveries, outbox);
- `handlers/mod.rs` no longer declares `auth` or `webhooks`.

Out of scope:

- extracting owned `AuthService`/`WebhookService` structs (the methods remain
  inherent on `WorkspaceServer` for now);
- domain events.

## Acceptance criteria

P1. **Given** auth routes, **when** they run, **then** login/logout/session
behavior is unchanged.

P2. **Given** webhook routes and the delivery worker, **when** they run, **then**
subscriptions, signatures, retries and dead letters are unchanged.

P3. **Given** the module tree, **when** a contributor looks for auth or webhook
logic, **then** it is under `server/services/` rather than `handlers/`.

P4. **Given** the existing auth/webhook tests, **when** the move is applied,
**then** they pass unchanged.

## Test scenarios

- `tests/auth.rs`
- `tests/webhooks.rs`
- `tests/smoke.rs`
- Full quality gate

## Design

- The methods remain `impl WorkspaceServer` so no call site changes.
- The module boundary is the seam for a later owned-service extraction.
- No route, DTO or persistence change.

## Plan / tasks

- [x] Move the auth and webhook modules
- [x] Update the module declarations
- [x] Run the full gate and commit
