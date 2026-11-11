# Deployment guide - ion-of-notes

This guide covers the self-hosted server container from `spec-050` and the
hardening/operations work in `spec-051`. The desktop app remains a local GUI;
only the server is deployed.

## 1. Security posture

The Compose stack is deliberately conservative:

- the published port binds to `127.0.0.1` by default;
- the container runs as UID 10001, not root;
- the root filesystem is read-only, with a small `/tmp` tmpfs;
- all Linux capabilities are dropped;
- `no-new-privileges` is enabled;
- the process receives `SIGTERM` and shuts down gracefully;
- logs are rotated by the Docker `json-file` driver.

For network access, put a TLS reverse proxy in front and keep the app bound to
loopback. Do not publish port 8787 directly to the internet.

## 2. Quick start

```sh
cp secrets/owner_password.txt.example secrets/owner_password.txt
# edit secrets/owner_password.txt and set a strong password
docker compose up --build -d
docker compose ps
curl http://127.0.0.1:8787/health
curl http://127.0.0.1:8787/ready
```

Expected health body:

```json
{"data":"ok","error":null}
```

`/health` is the liveness probe. `/ready` is the readiness probe used by the
container smoke test and orchestration; both return `200` once the router is
serving.

The first SQLite run seeds the owner account from the Docker secret at
`/run/secrets/owner_password` (`ION_OF_NOTES_OWNER_PASSWORD_FILE`). The
account is seeded once; changing the secret later does not change an existing
password.

To expose the server on a LAN or a reverse-proxy host, set the optional values
in `.env`:

```sh
ION_OF_NOTES_BIND=0.0.0.0
ION_OF_NOTES_PORT=8787
```

Only do this behind a firewall or a TLS proxy.

## 3. TLS with Caddy

`deploy/Caddyfile.example` is a minimal reverse proxy with automatic
certificates:

```text
your.domain.example {
    encode zstd gzip
    reverse_proxy 127.0.0.1:8787
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains"
        X-Content-Type-Options "nosniff"
        Referrer-Policy "no-referrer"
    }
}
```

Steps:

1. Copy the file to the Caddy configuration directory.
2. Replace `your.domain.example` with the public hostname.
3. Point DNS at the host and allow ports 80 and 443.
4. Reload Caddy and verify with `curl https://your.domain.example/health`.

If Caddy runs in the same Compose network instead of on the host, change the
upstream to the service name:

```text
reverse_proxy ion-of-notes:8787
```

The app itself does not terminate TLS; the proxy owns certificates and
redirects. Traefik, nginx and HAProxy work the same way with
`127.0.0.1:8787` (host) or `ion-of-notes:8787` (same Compose network) as the
upstream.

## 4. Data volume and permissions

The named volume is `ion-of-notes-data` and is mounted at `/data`. It contains
`ion-of-notes.sqlite3`, which holds the workspace, collaboration state,
accounts and sessions.

Inspect it:

```sh
docker volume inspect ion-of-notes-data
docker compose exec ion-of-notes ls -la /data
```

If you replace the named volume with a bind mount, give UID/GID 10001 write
access:

```sh
sudo chown -R 10001:10001 ./data
```

The read-only root filesystem is intentional. The app writes only to `/data`
and `/tmp`; do not add writable paths unless a custom command needs them.

## 5. Backup and restore

### Cold backup (recommended)

Stop the stack so SQLite is quiescent, archive the volume with a temporary
Alpine container, then start it again:

```sh
docker compose stop
mkdir -p backups
docker run --rm \
  -v ion-of-notes-data:/data:ro \
  -v "$PWD/backups:/backups" \
  alpine:3.20 \
  tar czf "/backups/ion-of-notes-data-$(date +%F-%H%M%S).tgz" -C /data .
docker compose start
```

PowerShell:

```powershell
docker compose stop
New-Item -ItemType Directory -Force backups | Out-Null
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
docker run --rm -v ion-of-notes-data:/data:ro -v "${PWD}/backups:/backups" `
  alpine:3.20 tar czf "/backups/ion-of-notes-data-$stamp.tgz" -C /data .
docker compose start
```

Keep backups outside the host or in a separate backup volume, and test a
restore regularly.

### Restore

Restoring replaces the current volume contents. Stop the stack first:

```sh
docker compose down
docker run --rm \
  -v ion-of-notes-data:/data \
  -v "$PWD/backups:/backups" \
  alpine:3.20 \
  sh -c 'rm -rf /data/* && tar xzf /backups/ion-of-notes-data-YYYY-MM-DD-HHMMSS.tgz -C /data && chown -R 10001:10001 /data'
docker compose up -d
curl http://127.0.0.1:8787/health
```

Then open the workspace and confirm the expected pages are present.

After a manual restore Docker may warn that the volume was not created by
Compose. The warning is harmless: the volume name and data are correct.

### Hot SQLite backup (optional)

If you need a backup without stopping the service, use a SQLite client that
supports the online backup API. With a temporary Alpine container:

```sh
docker run --rm \
  -v ion-of-notes-data:/data:ro \
  -v "$PWD/backups:/backups" \
  alpine:3.20 \
  sh -c 'apk add --no-cache sqlite >/dev/null && sqlite3 /data/ion-of-notes.sqlite3 ".backup /backups/ion-of-notes.sqlite3"'
```

A cold volume archive is simpler and is the supported default.

## 6. Secrets

- Compose mounts `secrets/owner_password.txt` at
  `/run/secrets/owner_password` and sets
  `ION_OF_NOTES_OWNER_PASSWORD_FILE=/run/secrets/owner_password`. The value is
  not in the environment and does not appear in `docker inspect`.
- Keep `secrets/owner_password.txt` out of version control and restrict it:
  `chmod 600 secrets/owner_password.txt`. The repository ignores everything
  under `secrets/` except the `.example` file.
- `ION_OF_NOTES_OWNER_PASSWORD` remains the explicit fallback for
  `docker run` and local `--serve`; the environment variable takes precedence
  when both are set.
- Rotating the secret does not change an existing seeded account. There is no
  password-reset flow yet; plan account changes as a data/account migration.

## 7. Upgrades and rollback

1. Take a backup (section 5).
2. Build or pull the new image.
3. Recreate the service and check health.

```sh
docker compose build --pull
docker compose up -d
docker compose ps
curl http://127.0.0.1:8787/health
docker compose logs --tail=100 ion-of-notes
```

The SQLite schema is migrated by the application on open. If an upgrade
fails, stop the stack, restore the backup volume, and start the previous image
tag. Keep the previous image available until the new one is verified.

## 8. Health, logs and troubleshooting

```sh
docker compose ps
docker inspect --format '{{.State.Health.Status}}' app-ion-of-notes-1
docker compose logs -f ion-of-notes
```

- `health: starting` means the start period has not elapsed yet; check logs.
- `permission denied` on `/data` means a bind mount is not owned by UID 10001.
- `read-only file system` means a custom command writes outside `/data` or
  `/tmp`; add a tmpfs only if you understand the path.
- Owner-password errors mean `secrets/owner_password.txt` is missing or empty,
  or the `ION_OF_NOTES_OWNER_PASSWORD` fallback is unset.
- Set `RUST_LOG=debug` temporarily for more detail; revert to `info` afterward.
- `GET /metrics` exposes `ion_of_notes_up`, `ion_of_notes_uptime_seconds` and
  `ion_of_notes_http_requests_total` in Prometheus text format. Keep it behind
  the proxy if request volume is sensitive.

## 9. Multi-arch images

`docker-bake.hcl` defines the `linux/amd64` and `linux/arm64` target. Validate
the definition with:

```sh
docker buildx bake --print server
```

Build and push both architectures with:

```sh
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t registry.example/ion-of-notes-server:1.0.0 \
  --push .
```

The build compiles the server-only binary with `--no-default-features`; no
eframe/egui/wgpu is included. Cross-architecture builds are slower because
`rusqlite` bundles and compiles SQLite for each target; native builders are
faster.

## 10. Security checklist

- [ ] `secrets/owner_password.txt` is strong and stored outside git.
- [ ] The secret file is `chmod 600` and not committed.
- [ ] The app is bound to `127.0.0.1` or a private network.
- [ ] A TLS reverse proxy terminates public traffic.
- [ ] Port 8787 is not exposed directly to the internet.
- [ ] Backups run on a schedule and restores are tested.
- [ ] The image is rebuilt for security updates and dependencies are reviewed.
- [ ] Base images and CI actions stay pinned; Renovate update PRs are reviewed.
- [ ] Log rotation and host disk monitoring are in place.
- [ ] The container remains non-root with `cap_drop: [ALL]` and
      `no-new-privileges`.
