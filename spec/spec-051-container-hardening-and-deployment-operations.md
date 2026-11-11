# spec-051 - Container hardening and deployment operations

- Status: implemented (2026-11-11)
- Related: `spec-050`, `docs/adr-003-self-hosted-collaborative-api.md`,
  `docs/architecture.md`, `README.md`

## Purpose and scope

`spec-050` made the server deployable with Compose. The container is still
permissive: the root filesystem is writable, all capabilities are kept, the
port is published on every interface, and there is no operations guide for
TLS, backups, secrets or upgrades. This spec hardens the runtime and documents
the operational path for a self-hosted deployment.

In scope:

- Bind the published port to `127.0.0.1` by default; allow
  `ION_OF_NOTES_BIND=0.0.0.0` for a LAN/reverse-proxy host.
- Run the container with a read-only root filesystem, a small `/tmp` tmpfs,
  `cap_drop: [ALL]`, `no-new-privileges`, `init: true`, a stop grace period and
  bounded JSON logs.
- Add a Caddy reverse-proxy/TLS example and a `docs/deployment.md` operations
  guide covering:
  - TLS termination and reverse proxy;
  - data volume, UID and bind-mount permissions;
  - cold backup and restore, plus a hot SQLite `.backup` option;
  - secret handling (`.env` permissions, Docker secrets as the next step);
  - upgrade and rollback;
  - health, logs and troubleshooting;
  - multi-arch image builds.
- Update the README, docs hub, glossary, requirements, spec index and changelog.

Out of scope:

- No PostgreSQL, password reset, registration or async webhook queue.
- No TLS termination inside the app; the operator's proxy terminates TLS.
- No automatic backup scheduler or hosted control plane.
- No Rust test-count change.

## Evidence

- `spec-050` added `compose.yml`, `Dockerfile`, the `ion-data` volume and
  `SIGTERM` handling.
- `README.md` still lists missing TLS/reverse-proxy/backup guidance in its
  current limitations.
- ADR-003 says TLS terminates at the operator's reverse proxy; the app itself
  needs no certificate code.
- The runtime image already runs as UID 10001 and exposes `/health`.
- The server writes only `/data`; a read-only root with a `/tmp` tmpfs should
  be sufficient.
- `rusqlite` is bundled; SQLite `.backup`/`VACUUM INTO` can be used without a
  runtime SQLite package.
- The existing Compose smoke test (health, page persistence, graceful stop)
  is the behavior contract for the hardened stack.

## Acceptance criteria

1. **Given** `compose.yml`, **when** `docker compose config` runs without
   overrides, **then** the published port is `127.0.0.1:8787:8787` and the
   service has a read-only rootfs, `/tmp` tmpfs, `cap_drop: [ALL]`,
   `no-new-privileges`, `init: true`, a stop grace period and log rotation.
2. **Given** `ION_OF_NOTES_BIND=0.0.0.0`, **when** Compose starts, **then** the
   port is published on all interfaces for a reverse proxy or LAN.
3. **Given** the hardened container, **when** it starts, **then** `/health`
   returns `200`, a page survives a restart on the same volume, and
   `docker stop` exits promptly.
4. **Given** `docs/deployment.md`, **when** an operator follows it, **then**
   they can back up and restore the volume, put the server behind Caddy TLS,
   rotate the owner password safely, upgrade the image and inspect health/logs.
5. **Given** `deploy/Caddyfile.example`, **when** `caddy validate` runs, **then**
   the example is syntactically valid.
6. **Given** the full gate, **when** fmt, both clippy passes, core + desktop
   tests, docs and `--screenshot` run, **then** all are green and the Rust test
   count stays 255.
7. **Given** the docs, **when** the spec completes, **then** the README,
   architecture, testing, requirements, glossary, spec index and changelog
   describe the hardened deployment.

## Test scenarios (written before implementation)

- **Compose config:** `docker compose config` shows the loopback bind and the
  hardening keys; `ION_OF_NOTES_BIND=0.0.0.0 docker compose config` shows the
  all-interface bind.
- **Hardened smoke:** `docker compose up -d --build`; `/health` returns `200`;
  `docker inspect` shows `ReadonlyRootfs=true`, `CapDrop=[ALL]`,
  `no-new-privileges`, and the configured stop grace period; create a page,
  restart, and read it back; `docker compose down`.
- **Backup/restore:** stop the stack, archive `ion-data`, restore it into a new
  volume, start the stack, confirm the page is present and write a new page to
  prove the restored files are owned by UID 10001.
- **Caddy example:** `caddy validate --config deploy/Caddyfile.example` in a
  `caddy:2` container.
- **Docs/lint:** `pwsh -File scripts/quality.ps1` and
  `pwsh -File scripts/check-docs.ps1` pass.

## Design

```text
compose.yml              loopback bind, read_only, tmpfs, cap_drop, no-new-privileges,
                         init, stop_grace_period, json-file log rotation
.env.example             ION_OF_NOTES_BIND, port, password
deploy/Caddyfile.example reverse proxy + automatic TLS
docs/deployment.md       operations guide: TLS, backups, secrets, upgrades,
                         health, logs, multi-arch
```

Compose hardening:

```yaml
ports:
  - "${ION_OF_NOTES_BIND:-127.0.0.1}:${ION_OF_NOTES_PORT:-8787}:8787"
read_only: true
tmpfs:
  - /tmp:size=64m
cap_drop:
  - ALL
security_opt:
  - no-new-privileges:true
init: true
stop_grace_period: 15s
logging:
  driver: json-file
  options:
    max-size: "10m"
    max-file: "3"
```

The server writes only to `/data`, so the read-only root is safe. The Caddy
example proxies `127.0.0.1:8787` and terminates TLS; operators can use Traefik
or another proxy with the same upstream. Backups are cold volume archives by
default; a hot `.backup` is documented for operators with a `sqlite3` client.

## Plan / tasks

- [x] Write this spec and record the red/green smoke plan.
- [x] Harden `compose.yml` and extend `.env.example`.
- [x] Add `deploy/Caddyfile.example`.
- [x] Add `docs/deployment.md` and link it from the docs hub/README.
- [x] Run the Compose config, hardened smoke, backup/restore and Caddy checks.
- [x] Run fmt, both clippy passes, core + desktop tests, docs and screenshot.
- [x] Update `AGENTS.md`, README, architecture, testing, requirements, glossary,
      docs hub, spec index and changelog.
- [x] Commit the whole `app/` tree naming this spec.
