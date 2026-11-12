# spec-067 - Runtime smoke tests

- Status: implemented (2026-11-12)
- Related: `spec-038`, `spec-050`, `spec-051`, `spec-052`, `spec-056`
- Reference: `docs/testing.md`, `docs/deployment.md`

## Purpose and scope

Unit and integration tests exercise the router and core in-process. They do
not prove that the shipped server binary starts, binds a real port, answers
the operational endpoints, serves the v2 contract with headers, persists to
SQLite, or survives a Compose start. This spec adds the first runtime smoke
tests for those seams.

Initial scope:

- `tests/smoke.rs`: a real Axum listener on an ephemeral port with `/health`,
  `/ready`, `/metrics` and an authenticated v2 workspace request.
- `scripts/smoke-server.ps1`: build/run the headless `ion-of-notes-server`
  binary with a temporary SQLite database, poll the endpoints, verify a page
  round-trip, stop the process, and clean up.
- `scripts/smoke-docker.ps1`: opt-in Docker Compose smoke for the development
  stack: build, start, health/ready/metrics, a page write/read, stop, and
  verify the named volume persists the page across a recreate.
- AGENTS rule: a smoke test is required when a feature touches startup wiring,
  a real listener, deployment, cross-process persistence or another shipped
  artifact; if it needs Docker/TLS/network it is opt-in and documented.

Out of scope for this first pass:

- Public TLS/ACME smoke (needs a domain or local CA trust).
- Multi-arch build execution (`docker buildx bake` is validated separately).
- A CI matrix that runs every smoke test on every platform.

## Acceptance criteria

S1. **Given** the crate, **when** `cargo test --test smoke` runs, **then** a
real listener serves `/health`, `/ready`, `/metrics` and an authenticated v2
`/api/v2/workspace` response with a request ID, and the listener is stopped.

S2. **Given** the headless binary, **when** `scripts/smoke-server.ps1` runs,
**then** it builds if needed, starts on a free loopback port with a temporary
SQLite database, checks health/ready/metrics, creates and reads a page through
the API, and exits non-zero on any failure while always stopping the process.

S3. **Given** Docker is available, **when** `scripts/smoke-docker.ps1` runs,
**then** it builds `compose.yml`, waits for the healthcheck, checks
health/ready/metrics, writes and reads a page, recreates the service and
verifies the page survived in the `ion-data` volume.

S4. **Given** Docker is unavailable, **when** the Docker smoke runs, **then**
it exits with a clear opt-in message rather than a stack trace.

S5. **Given** the full gate, **when** it runs, **then** the default gate stays
fast and does not start Docker; the smoke commands are documented and runnable
on demand.

## Test scenarios

- `tests/smoke.rs`: real listener, HTTP status/body checks, v2 auth headers,
  request ID, clean shutdown.
- `scripts/smoke-server.ps1`: temp database, port collision fallback, process
  cleanup on failure, page create/read.
- `scripts/smoke-docker.ps1`: compose up/down, health wait, volume persistence
  across `docker compose up --force-recreate`.
- Full gate remains green.

## Design

- `tests/smoke.rs` uses `server::web::router` with a `TcpListener` bound to
  `127.0.0.1:0`, exactly like the remote tests, but keeps the requests at the
  HTTP boundary with `ureq` so the shipped adapter is exercised.
- The PowerShell scripts own process/compose lifecycle. They print each step,
  use `try/finally` for cleanup, and never leave a listener or stack behind.
- Docker smoke uses the existing `compose.yml`; it does not invent a second
  stack. It is opt-in because it builds an image.
- The default `quality.ps1` stays unchanged; smoke tests are explicit commands.

## Plan / tasks

- [x] Add `tests/smoke.rs` and run it red/green.
- [x] Add `scripts/smoke-server.ps1` and run it against the headless binary.
- [x] Add `scripts/smoke-docker.ps1` and run it with Docker.
- [x] Document the commands in `docs/testing.md` and `AGENTS.md`.
- [x] Full gate and changelog.
