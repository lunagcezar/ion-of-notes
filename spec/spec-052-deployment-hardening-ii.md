# spec-052 - Deployment hardening II

- Status: implemented (2026-11-11)
- Related: `spec-050`, `spec-051`, `docs/deployment.md`, `compose.yml`

## Purpose and scope

`spec-051` hardened the container and documented the manual deployment path.
This follow-up adds the operational endpoints, secret handling and build
surface needed for a production-style self-hosted deployment.

In scope:

- Add `/ready` and `/metrics` to the Axum adapter.
  - `/ready` is a readiness probe distinct from the liveness `/health`.
  - `/metrics` exposes Prometheus text for up, uptime and HTTP request count.
- Support `ION_OF_NOTES_OWNER_PASSWORD_FILE` so Docker secrets can provide the
  owner password without putting it in the environment.
- Switch the default Compose stack to a Docker secret for the owner password;
  keep the environment variable as a local/`docker run` fallback.
- Add `docker-bake.hcl` for multi-architecture builds.
- Add a GitHub Actions container smoke job (build image, `/ready`, `/health`,
  `/metrics`, cleanup).
- Update README, deployment guide, architecture, testing, requirements,
  glossary, spec index and changelog.

Out of scope:

- No PostgreSQL, registration, password reset or account-management UI.
- No automatic backup scheduler; the cold/hot backup commands stay in
  `docs/deployment.md`.
- No TLS termination inside the app; the reverse proxy owns certificates.
- No Rust test-count change beyond the new endpoint tests.

## Evidence

- `spec-051` already documents `/health`, the `ion-of-notes-data` volume,
  loopback binding, read-only root and `SIGTERM`.
- `src/server/web.rs` owns the Axum router and currently has only the `/health`
  fallback plus the WebSocket route.
- `src/app.rs` seeds the owner account from `ION_OF_NOTES_OWNER_PASSWORD`;
  Docker secrets mount files under `/run/secrets`.
- `tests/web.rs` already exercises the router through `tower::ServiceExt`, so
  the new endpoints can be tested without a listener.
- `docker buildx` can build the existing server-only Dockerfile for
  `linux/amd64` and `linux/arm64`; the Dockerfile has no architecture-specific
  steps.

## Acceptance criteria

1. **Given** a running server, **when** `GET /ready` is requested, **then** it
   returns `200` with a JSON envelope and is independent of `/health`.
2. **Given** a running server, **when** `GET /metrics` is requested, **then**
   it returns Prometheus text containing `ion_of_notes_up`,
   `ion_of_notes_uptime_seconds` and `ion_of_notes_http_requests_total`.
3. **Given** `ION_OF_NOTES_OWNER_PASSWORD_FILE`, **when** the first SQLite run
   seeds the owner, **then** the file contents (trimmed) are used; the
   environment variable still takes precedence when both are set.
4. **Given** `compose.yml`, **when** it starts, **then** the owner password is
   mounted from `secrets/owner_password.txt` and is not present in the
   environment or `docker inspect`.
5. **Given** `docker buildx bake`, **when** it runs, **then** it builds the
   `linux/amd64` and `linux/arm64` targets from one definition.
6. **Given** the GitHub container job, **when** CI runs, **then** it builds the
   image, waits for `/ready`, checks `/health` and `/metrics`, and removes the
   container and volume.
7. **Given** the full gate, **when** fmt, both clippy passes, core + desktop
   tests, docs and the preview screenshots run, **then** all are green.
8. **Given** the docs, **when** the spec completes, **then** the deployment
   guide and README describe the secret file, readiness, metrics and multi-arch
   build.

## Test scenarios (written before implementation)

- **Red:** add the `/ready` and `/metrics` routes and the password-file helper
  before their handlers/readers exist; `cargo test --test web` and
  `cargo test --test auth` fail to compile or fail the new assertions.
- **Green:** `ready_route_reports_ready` and
  `metrics_route_exposes_prometheus_text` pass; the password-file test reads a
  temp file, trims the trailing newline and rejects an empty file.
- **Existing behavior:** the core + desktop suites stay at 258 tests, plus the
  new endpoint/auth tests.
- **Compose:** `docker compose config` shows the `owner_password` secret and no
  password environment value; a smoke run with a temporary secret file starts
  and serves `/ready`.
- **Bake:** `docker buildx bake --print server` resolves the two platforms.
- **CI:** the GitHub container job is reviewed in the workflow and its commands
  are run locally where Docker is available.
- **Lint/docs:** `pwsh -File scripts/quality.ps1` and
  `pwsh -File scripts/check-docs.ps1` pass.

## Design

```text
src/server/web.rs
  GET /ready   -> 200 {"data":"ready","error":null}
  GET /metrics -> text/plain; version=0.0.4
                  ion_of_notes_up 1
                  ion_of_notes_uptime_seconds <n>
                  ion_of_notes_http_requests_total <n>

src/auth.rs
  read_password_file(path) -> trimmed contents, empty is an error

src/app.rs
  owner_password(): env var first, then *_FILE, else the development default

compose.yml
  secrets: owner_password -> /run/secrets/owner_password
  environment: ION_OF_NOTES_OWNER_PASSWORD_FILE=/run/secrets/owner_password

secrets/owner_password.txt.example   placeholder + instructions
docker-bake.hcl                      linux/amd64 + linux/arm64 target
.github/workflows/ci.yml             container build + smoke job
```

The `/metrics` counters live in the Axum `AppState` (`Instant` start time and an
`AtomicU64` request counter); `/health` and `/ready` are explicit routes, and
`/metrics` is excluded from the request counter so scraping does not inflate
it. The password file is read only during the first SQLite seed; the
environment variable remains the explicit override for `docker run` and local
development.

## Plan / tasks

- [x] Write this spec and record the red step.
- [x] Add `/ready`, `/metrics` and the request counter with tests.
- [x] Add `read_password_file`, `*_FILE` support and tests.
- [x] Switch Compose to the Docker secret and add the example file.
- [x] Add `docker-bake.hcl` and the multi-arch docs.
- [x] Add the GitHub container smoke job.
- [x] Run tests, clippy, fmt, docs, the Compose smoke and the preview screenshots.
- [x] Update `AGENTS.md`, README, deployment guide, architecture, testing,
      requirements, glossary, spec index and changelog.
- [x] Commit the whole `app/` tree naming this spec.
