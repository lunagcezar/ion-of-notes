# spec-038 — Axum web server and contract suite

## Purpose and scope

The optional `--serve` mode uses a small hand-written HTTP/1.1 loopback
listener. It is useful for tests and a local demo, but it is not a production
web server: it has no graceful shutdown, no structured request logging, no
Tower middleware and no path to WebSocket or TLS adapters. This spec adds an
Axum/Tokio/Tower HTTP adapter behind the existing `WorkspaceApi` seam and a
contract suite that drives the real `Client` and the HTTP adapter together. It
also completes Track B item B5 from `spec-016`.

In scope:

- an Axum router that converts HTTP requests into `ApiRequest` and calls the
  existing `WorkspaceServer`;
- `--serve` uses the Axum adapter; the hand-written listener remains available
  for parser tests and as a fallback;
- Tokio runtime, graceful shutdown on Ctrl+C, `tower-http` tracing and body
  limits;
- `X-Request-Id` and `Retry-After` response headers;
- a contract suite in `tests/web.rs` covering v1 health, v2 auth/versioning,
  errors, pagination, webhooks and limits through the Axum adapter;
- documentation and status updates.

Out of scope:

- sessions, password hashing and durable SQL storage (spec-039);
- WebSocket/CRDT real-time transport (spec-040);
- TLS termination, reverse-proxy configuration and deployment manifests;
- a browser frontend;
- changing v1 or v2 paths, payloads or status codes.

## Evidence

- `docs/adr-003-self-hosted-collaborative-api.md` — the self-hosted server
  target.
- `docs/adr-006-public-api-alignment.md` — v2 contract and `/api/v2`.
- `spec/spec-016-architecture-follow-up-and-observation.md` — Track B, B5.
- Current code: `src/server.rs` (`WorkspaceApi`, `WorkspaceServer`),
  `src/server/http.rs` (hand-written loopback adapter), `src/main.rs`
  (`--serve`), `tests/contract.rs` (contract suite against the core).

## Acceptance criteria

1. **Given** `--serve 127.0.0.1:8787`, **when** the process starts, **then** an
   Axum server listens on that address and answers `/health` with `200`.
2. **Given** a v1 request, **when** it is sent through Axum, **then** the
   response body and status match the in-process server.
3. **Given** a v2 request without the version header, **when** it is sent,
   **then** the adapter returns `400 missing_version`; without a bearer token it
   returns `401 unauthorized`.
4. **Given** a v2 response, **when** it succeeds, **then** the `X-Request-Id`
   header matches the request ID in the body/log.
5. **Given** a `429`, `503` or `529` response, **when** it is sent over HTTP,
   **then** `Retry-After` is present and agrees with
   `additional_data.retry_after`.
6. **Given** a query string, **when** the request reaches the adapter, **then**
   the path and query are preserved exactly.
7. **Given** a 501 KB request body, **when** it is sent, **then** the server
   returns `400 validation_error` from the v2 size check, not a transport
   failure.
8. **Given** Ctrl+C, **when** the server is running, **then** it stops accepting
   connections and exits cleanly.
9. **Given** the existing suite, **when** it runs, **then** all tests stay green
   and the new `tests/web.rs` contract suite passes.
10. **Given** the dependency direction, **when** the adapter is added, **then**
    no Axum type appears in `model`, `client`, `collab` or the core `server`
    logic.

## Test scenarios (written before implementation)

- `tests/web.rs` (new):
  - `/health` through Axum;
  - v1 pages through Axum;
  - v2 version/auth failures;
  - v2 workspace round-trip and `X-Request-Id`;
  - `429` with `Retry-After`;
  - 501 KB body → `400 validation_error`;
  - query-string preservation and pagination;
  - webhook create/list/delete through Axum;
  - unknown route → structured `404`.
- `tests/contract.rs`: unchanged core contract suite, still green.
- `tests/server.rs`: the hand-written parser/formatter tests stay green.

## Design

```text
main --serve
   └── server::web::serve_axum
          ├── tokio runtime
          ├── axum::Router fallback -> ApiRequest
          ├── tower-http TraceLayer + DefaultBodyLimit
          └── Arc<Mutex<WorkspaceServer>> + spawn_blocking
```

- `src/server/web.rs` owns the Axum adapter. It is an infrastructure module;
  the domain and application core stay transport-neutral.
- The adapter converts `http::Request` into `ApiRequest` (method, path+query,
  lowercase headers, UTF-8 body) and `ApiResponse` back into an HTTP response.
- The synchronous `WorkspaceApi::handle` runs in `tokio::task::spawn_blocking`
  behind a mutex, so the reactor is never blocked.
- `X-Request-Id` and `Retry-After` are copied from the existing response model;
  the error body remains the single source of truth for the retry wait.
- The old `server/http.rs` listener remains for parser tests and as a minimal
  fallback; `main.rs` uses the Axum adapter.

## Plan / tasks

- [x] Spec + failing tests (red).
- [x] Add Axum, Tokio, Tower, Tower-HTTP, tracing dependencies.
- [x] Implement `server/web.rs` router and `serve_axum`.
- [x] Switch `--serve` to the Axum adapter; keep the old listener.
- [x] Add `tests/web.rs` contract suite.
- [x] Update `docs/api-contract.md`, `docs/architecture.md`,
  `docs/testing.md`, `docs/index.md`, `AGENTS.md`, `README.md`,
  `spec/README.md`, `CHANGELOG.md`.
- [x] Mark `spec-016` B5 done.
- [x] Build/test/clippy/fmt/docs/screenshot, commit.

## Result

- Added `src/server/web.rs`: an Axum router and `serve_axum` adapter behind the
  existing `WorkspaceApi` seam, with `spawn_blocking`, a shared mutex,
  `TraceLayer`, `DefaultBodyLimit` and graceful Ctrl+C shutdown.
- `--serve` now uses Axum/Tokio; the hand-written loopback listener remains for
  parser/formatter tests and as a minimal fallback.
- HTTP responses carry `Content-Type`, `X-Request-Id` and `Retry-After` (from
  the structured error body).
- Added `tests/web.rs`: 9 contract tests covering health, v1 pages, v2
  version/auth, request IDs, `Retry-After`, 501 KB validation, query strings,
  pagination, webhooks and structured 404s.
- Added `docs/adr-007-production-web-server.md` for the Axum/Tokio/Tower
  decision.
- The suite is 221 tests; B5 is complete.

## Requirement mapping

| Requirement | How |
| ----------- | --- |
| NFR (production transport) | Axum + Tokio + Tower HTTP adapter |
| NFR (observability) | request IDs, tracing, structured errors |
| NFR (reliability) | graceful shutdown, body limits, `Retry-After` |
| B5 (contract suite) | `tests/web.rs` + existing `tests/contract.rs` |
