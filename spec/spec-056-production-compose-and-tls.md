# spec-056 - Production Compose and TLS manifest

- Status: implemented (2026-11-11)
- Related: `spec-050`, `spec-051`, `spec-052`, `docs/deployment.md`

## Purpose and scope

The repository has a hardened development Compose stack and a Caddy example,
but no production manifest that wires the server and a TLS reverse proxy
together. Operators still have to assemble the network, ports, volumes and
certificate configuration themselves. This spec adds one production Compose
file with Caddy and an optional scheduled backup profile.

In scope:

- Add `compose.prod.yml`: server + Caddy, no host port on the app, Caddy owns
  80/443, named data/config volumes, Docker secret, health-gated dependency,
  log rotation and the same app hardening as the development stack.
- Add `deploy/Caddyfile` using `{$ION_OF_NOTES_DOMAIN}` and
  `deploy/Caddyfile.local` for a local `tls internal` test.
- Add `.env.prod.example` with the domain and image/backup settings.
- Add an optional `backup` profile that performs an online SQLite `.backup`
  daily and prunes old archives.
- Document the production flow, local TLS test, certificate storage and backup
  profile.
- Update the README, architecture, deployment guide, requirements, glossary,
  spec index and changelog.

Out of scope:

- No hosted control plane, Kubernetes manifests or cloud load balancer.
- No automatic DNS provider integration; the operator points DNS at the host.
- No change to the application API, Rust test count or desktop behavior.

## Evidence

- `compose.yml` already has the app hardening, secret, `ion-data` volume and
  `/health` healthcheck.
- `deploy/Caddyfile.example` proves the reverse-proxy shape but is not wired
  into a Compose service.
- `docs/deployment.md` documents cold/hot backups, but there is no scheduled
  profile.
- `spec-052` added `/ready`, `/metrics` and the Docker secret.
- Caddy supports environment placeholders in the Caddyfile and automatic
  certificates; the official image is available for a pinned digest.

## Acceptance criteria

1. **Given** `compose.prod.yml` and a domain, **when** `docker compose config`
   runs, **then** Caddy publishes 80/443, the app has no host port and only
   Caddy can reach `ion-of-notes:8787`.
2. **Given** a valid domain and DNS, **when** the stack starts, **then** Caddy
   obtains certificates, proxies to the app and the app is healthy.
3. **Given** `deploy/Caddyfile.local`, **when** it is used for a local test,
   **then** Caddy uses its internal CA and proxies `localhost`.
4. **Given** `--profile backup`, **when** the backup service runs, **then** it
   writes an online SQLite backup and prunes archives older than the retention
   window.
5. **Given** the full gate, **when** fmt, both clippy passes, core + desktop
   tests, docs and the preview screenshots run, **then** all are green.
6. **Given** the docs, **when** the spec completes, **then** the README and
   deployment guide describe the production stack, local TLS test and backup
   profile.

## Test scenarios (written before implementation)

- **Config:** `ION_OF_NOTES_DOMAIN=notes.example.com docker compose -f
  compose.prod.yml config` shows Caddy ports, no app host port, the secret and
  the health-gated dependency.
- **Backup profile:** `docker compose -f compose.prod.yml --profile backup
  config` includes the backup service and `backups` mount.
- **Caddy:** `caddy validate` accepts both Caddyfiles with the domain env set.
- **Local:** `deploy/Caddyfile.local` contains `tls internal` and proxies the
  service name.
- **Full gate:** `pwsh -File scripts/quality.ps1` and the preview screenshots.

## Design

```text
compose.prod.yml          app + caddy + optional backup profile
.env.prod.example         ION_OF_NOTES_DOMAIN, image, backup retention
deploy/Caddyfile          {$ION_OF_NOTES_DOMAIN} -> ion-of-notes:8787
deploy/Caddyfile.local    localhost + tls internal
```

The app uses `expose: ["8787"]` only; Caddy publishes 80/443 and reaches the
app by service name. Caddy stores certificates in `caddy-data` and config in
`caddy-config`. The app keeps `read_only`, `cap_drop`, `no-new-privileges`,
`/tmp` tmpfs, `stop_grace_period` and log rotation. The backup profile mounts
`ion-data` read-only and `./backups`, runs `sqlite3 .backup` daily, and prunes
archives older than `ION_OF_NOTES_BACKUP_RETENTION_DAYS` (default 7).

## Plan / tasks

- [x] Write this spec and record the config checks.
- [x] Add `compose.prod.yml` and `.env.prod.example`.
- [x] Add `deploy/Caddyfile` and `deploy/Caddyfile.local`.
- [x] Add the optional backup profile.
- [x] Validate Compose configs and both Caddyfiles.
- [x] Update README, architecture, deployment, requirements, glossary,
      spec index and changelog.
- [x] Run the full gate and commit.
