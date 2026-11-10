# spec-050 - Server-only binary and Compose deployment

- Status: implemented (2026-11-10)
- Related: `spec-048`, `docs/adr-003-self-hosted-collaborative-api.md`,
  `docs/architecture.md`, `docs/testing.md`

## Purpose and scope

The self-hosted server can already run with `--serve`, but the only binary is
`ion-of-notes`, which requires the default-on `desktop` feature. A container
therefore links eframe/egui/wgpu even though the server never opens a window.
This spec adds a slim server entry point and a Compose deployment for local and
self-hosted use.

In scope:

- Add `src/bin/ion-of-notes-server.rs`, built without the desktop feature.
- Extract the shared serve wiring into `app::serve(addr, database)` so the
  desktop binary and the server binary use one composition path.
- Make the Axum adapter stop gracefully on `SIGTERM` as well as Ctrl+C, so
  `docker stop`/`docker compose down` are clean.
- Add a multi-stage `Dockerfile` and `.dockerignore`.
- Add `compose.yml` with a named SQLite volume, required owner password,
  port mapping and healthcheck, plus `.env.example`.
- Add a Docker smoke check to the docs and CI guidance; keep the Rust test
  suite unchanged.
- Update `AGENTS.md`, architecture, testing, requirements, glossary, README,
  spec index and changelog.

Out of scope:

- No PostgreSQL, reverse proxy, TLS termination or password-reset flow.
- No desktop-in-container support; the GUI remains a local application.
- No new test framework and no change to the API contract.
- No renumbering of `spec-049` (still reserved for the UI/client split).

## Evidence

- `src/main.rs` imports `ion_of_notes::ui::NotionApp` and `eframe`; the binary
  declares `required-features = ["desktop"]` in `Cargo.toml`.
- `src/app.rs` already owns the JSON/SQLite composition (`build_server`,
  `build_server_sqlite`) and `src/server/web.rs` owns `serve_axum`.
- `/health` is served by the existing Axum fallback and is already used by the
  tests and README.
- `ION_OF_NOTES_OWNER_PASSWORD` seeds the owner account on the first SQLite
  run; the development default is `change-me-now` and must not be the
  container default.
- `store::data_path()` uses `LOCALAPPDATA` or `HOME`; the container uses
  `--database /data/ion-of-notes.sqlite3` so the named volume is the only
  persistent state.
- `shutdown_signal()` currently listens only for Ctrl+C; Docker sends
  `SIGTERM`, so the process would wait for the 10-second kill timeout.
- The Linux `quality.ps1` gate already passes in a container-like environment
  (spec-048 validation), so a server-only build is the missing deployment
  piece.

## Acceptance criteria

1. **Given** `cargo build --release --no-default-features --bin ion-of-notes-server`,
   **when** it completes, **then** the binary serves the same API as
   `ion-of-notes --serve` and does not link eframe/egui/wgpu.
2. **Given** the desktop binary and the server binary, **when** `--serve` is
   used, **then** both call the same `app::serve` composition function.
3. **Given** a running container, **when** `/health` is requested, **then** it
   returns `200` and the response is the existing health payload.
4. **Given** a named `ion-data` volume, **when** the container is recreated,
   **then** the SQLite workspace and collaboration state survive.
5. **Given** `docker stop` or `docker compose down`, **when** the process
   receives `SIGTERM`, **then** it shuts down without waiting for the kill
   timeout.
6. **Given** `compose.yml`, **when** `ION_OF_NOTES_OWNER_PASSWORD` is missing,
   **then** Compose refuses to start instead of silently using
   `change-me-now`.
7. **Given** the full gate, **when** fmt, both clippy passes, core + desktop
   tests, docs and `--screenshot` run, **then** all are green and the Rust test
   count is unchanged.
8. **Given** the docs, **when** the spec completes, **then** the README,
   architecture, testing, requirements, glossary, spec index and changelog
   describe the server-only binary and Compose stack.

## Test scenarios (written before implementation)

- **Red step:** add `src/bin/ion-of-notes-server.rs` that calls
  `app::serve(...)` before the function exists. Run
  `cargo build --no-default-features --bin ion-of-notes-server` and observe the
  unresolved `app::serve` compile failure.
- **Green build:** the same command succeeds; `cargo tree
  --no-default-features` contains no `eframe`/`egui`/`wgpu`.
- **Existing behavior:** `cargo test --all-targets --no-default-features`
  (229) and the four desktop test crates (26) stay green.
- **Docker smoke:** `docker build -t ion-of-notes-server:test .`; run it with a
  temporary volume and `-p 18787:8787`; `curl /health` returns `200`; stop it
  with `docker stop` and verify the process exits promptly.
- **Persistence:** create a page or write through the API, remove the
  container, start a new one on the same volume, and read the page back.
- **Compose config:** `docker compose config` resolves the service, volume and
  required password; `docker compose up --build -d` becomes healthy.
- **Docs/lint:** `pwsh -File scripts/quality.ps1` and
  `pwsh -File scripts/check-docs.ps1` pass.

## Design

```text
src/bin/ion-of-notes-server.rs   server-only entry point
src/app.rs                       pub fn serve(addr, database)
src/main.rs                      Mode::Serve delegates to app::serve
src/server/web.rs                SIGTERM + Ctrl+C graceful shutdown
Dockerfile                       rust builder -> debian:bookworm-slim runtime
.dockerignore                    excludes target, docs, spec, screenshots, shots
compose.yml                      service, ion-data volume, healthcheck, env
.env.example                     required owner password + optional port/log
```

Server binary:

```rust
let args = Args::parse(std::env::args());
let Mode::Serve { addr } = args.mode else {
    eprintln!("usage: ion-of-notes-server --serve <addr> [--database <path>]");
    std::process::exit(2);
};
app::serve(&addr, args.database)?;
```

Dockerfile:

```dockerfile
FROM rust:1.99-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY locales ./locales
COPY tests ./tests
RUN cargo build --release --locked --no-default-features --bin ion-of-notes-server

FROM debian:bookworm-slim AS runtime
# ca-certificates + curl + passwd; rusqlite is bundled.
COPY --from=builder /src/target/release/ion-of-notes-server /usr/local/bin/
USER ion
VOLUME ["/data"]
EXPOSE 8787
HEALTHCHECK ... CMD curl -fsS http://127.0.0.1:8787/health
ENTRYPOINT ["ion-of-notes-server"]
CMD ["--serve", "0.0.0.0:8787", "--database", "/data/ion-of-notes.sqlite3"]
```

Compose uses `ION_OF_NOTES_OWNER_PASSWORD` with the `:?` required-variable
form, maps `${ION_OF_NOTES_PORT:-8787}:8787`, and mounts `ion-data:/data`.
The image healthcheck is inherited by Compose; `docker compose down` keeps the
volume, `down -v` removes it.

## Plan / tasks

- [x] Write this spec and record the red compile step.
- [x] Add `app::serve` and make `main.rs` delegate to it.
- [x] Add `src/bin/ion-of-notes-server.rs` and keep `cargo run` on the
      desktop with `default-run = "ion-of-notes"`.
- [x] Handle `SIGTERM` on Unix in `shutdown_signal`.
- [x] Add `Dockerfile`, `.dockerignore`, `compose.yml`, `.env.example` and
      `.gitignore` entries.
- [x] Run the red/green build, existing tests, both clippy passes and fmt.
- [x] Build and smoke-test the container; verify health, persistence and
      graceful stop; validate `docker compose config`.
- [x] Update `AGENTS.md`, README, architecture, testing, requirements, glossary,
      spec index and changelog.
- [x] Commit the whole `app/` tree naming this spec.
