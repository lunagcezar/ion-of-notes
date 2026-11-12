# spec-082 - Request pipeline stages

- Status: in progress (2026-11-12)
- Related: `spec-030`, `spec-037`, `spec-073`, `spec-078`
- Review sources: Claude (pipeline stages) and ChatGPT (route policy/pipeline)

## Purpose and scope

`handle_v2_inner` still runs version, login, authentication, rate limiting,
size limits, conflict checks, block limits, capability checks, authorization
and resource dispatch in one long function. The 429 response is built twice
and the ordering rules are implicit.

This spec extracts the cross-cutting stages into small functions with one
return type, so the order is explicit and each stage can be tested.

Scope:

- version check;
- login throttle + login dispatch;
- bearer authentication;
- overload / rate / size / conflict / block-limit checks;
- capability check;
- authorization check;
- a small `run_stage` helper that keeps the pipeline readable.

Out of scope:

- typed handler errors (`ApiResult`) and converting every handler;
- splitting `WorkspaceServer` into services;
- domain events.

## Acceptance criteria

P1. **Given** a request without the version header, **when** it has no token,
**then** the version error wins (400 before 401).

P2. **Given** a valid version and an invalid token, **when** the server is
overloaded, **then** the authentication error wins (401 before 529).

P3. **Given** a valid connection, **when** the rate budget is exhausted, **then**
the 429 response is built by one helper.

P4. **Given** a valid connection and a known route, **when** the capability is
missing, **then** the capability error wins before dispatch.

P5. **Given** a page-scoped route, **when** the principal has no share, **then**
the authorization error wins before the handler.

P6. **Given** the existing v2 contract tests, **when** the pipeline is
refactored, **then** status codes, envelopes and request IDs are unchanged.

## Test scenarios

- `tests/contract.rs` (existing version/auth/error tests)
- `tests/security.rs` (auth boundary)
- `tests/authorization.rs` (owner/page checks)
- `tests/web.rs` (Axum adapter)
- New ordering assertions in `tests/contract.rs`

## Design

- `routes/pipeline.rs` owns the stage functions:
  - `check_version`;
  - `authenticate`;
  - `check_login_rate`;
  - `check_runtime_limits`;
  - `check_capability`;
  - `check_authorization`.
- Each returns `Result<T, ApiResponse>`; the caller uses `?`.
- `rate_limit_response` is shared by login and authenticated requests.
- `handle_v2_inner` becomes a short sequence of stages plus the resource
  dispatch match.

## Plan / tasks

- [ ] Write spec-082 and failing ordering tests (red)
- [ ] Extract the stage functions
- [ ] Keep the login branch and dispatch in `v2.rs`
- [ ] Run the full gate and commit
