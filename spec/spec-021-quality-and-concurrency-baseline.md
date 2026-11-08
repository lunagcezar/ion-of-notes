# spec-021 — Quality and concurrency baseline

## Purpose and scope

This spec closes the last Priority 1 items:

1. **Client encode errors:** serialization failures must return an explicit
   `ClientError::Encode`, never an empty request body.
2. **Concurrent loopback listener:** `--serve` must accept independent clients
   concurrently instead of handling one connection at a time.
3. **Formatting baseline:** `cargo fmt --check` must pass.
4. **CI:** a workflow runs formatting, clippy, tests and a release build.

Out of scope:

- authentication/session middleware;
- a production Axum/PostgreSQL server;
- rate limiting, TLS termination or reverse-proxy configuration;
- changing the public REST contract.

## Evidence

- `src/client.rs`: every mutation body uses
  `serde_json::to_string(...).unwrap_or_default()`, so an encode failure sends
  an empty body and produces a confusing server error.
- `src/server/http.rs`: `serve_forever` handles one connection per loop
  iteration.
- No `.github/` workflow exists.
- `cargo fmt --check` fails on pre-existing formatting drift.
- `spec/spec-016-architecture-follow-up-and-observation.md` A7, A8, A10 plan
  these items.

## Acceptance criteria

1. **Given** a value whose `Serialize` implementation fails, **when** the
   client encodes a request body, **then** it returns `ClientError::Encode`
   with the serializer message.
2. **Given** any client mutation method, **when** it builds a body, **then** it
   uses the checked encoder; no `unwrap_or_default` remains in `client.rs`.
3. **Given** two clients connected to `--serve`, **when** both send a request,
   **then** both receive `200` without one waiting for the other's I/O.
4. **Given** shared server state, **when** requests run concurrently, **then**
   access is serialized through a mutex; no data race or torn write.
5. **Given** a clean checkout, **when** `cargo fmt --check` runs, **then** it
   succeeds.
6. **Given** a push or pull request, **when** CI runs, **then** it checks
   formatting, clippy, tests and a release build.

## Test scenarios (written before implementation)

- `tests/client.rs`: a deliberately failing `Serialize` type returns
  `ClientError::Encode`.
- `tests/server.rs`: `serve_on` handles two concurrent TCP connections and both
  receive `HTTP/1.1 200`.
- Existing 96 tests stay green.
- `cargo fmt --check`, clippy and the screenshot stay green/unchanged.

## Design

- Add `ClientError::Encode(String)` and a public
  `client::encode_body<T: Serialize>(&T)` helper; use it in every mutation.
- Add `WorkspaceStore: Send + Sync` and `CollabStore: Send + Sync` so the
  server can be shared across threads.
- Split `serve_once` into a read phase and a handle/write phase. `serve_on`
  reads the request first, locks the server only for `handle`, then writes the
  response. `serve_forever` binds and delegates to `serve_on`.
- Run `cargo fmt` once as a formatting-only commit.
- Add `.github/workflows/ci.yml` for Windows (`windows-latest`):
  `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --all-targets`, `cargo build --release`.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `ClientError::Encode` + checked encoder
- [x] `serve_on` + concurrent connection handling
- [x] `cargo fmt` baseline (separate commit)
- [x] CI workflow
- [x] Docs, changelog, build/test/clippy/screenshot, commit
