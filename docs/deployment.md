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
cp .env.example .env
# edit .env and set a strong ION_OF_NOTES_OWNER_PASSWORD
docker compose up --build -d
docker compose ps
curl http://127.0.0.1:8787/health
```

Expected health body:

```json
{"data":"ok","error":null}
```

The first SQLite run seeds the owner account from
`ION_OF_NOTES_OWNER_PASSWORD`. The account is seeded once; changing the
variable later does not change an existing password.

To expose the server on a LAN or a reverse-proxy host, set:

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

- Keep `.env` out of version control and restrict it: `chmod 600 .env`.
- `ION_OF_NOTES_OWNER_PASSWORD` is required by Compose; the stack refuses to
  start while it is unset.
- For production, inject the variable from your secret manager or use Docker
  secrets with a small entrypoint that exports the file contents. The app
  currently reads the environment variable directly; a native
  `*_PASSWORD_FILE` option is a future improvement.
- Rotating the environment variable does not change an existing seeded
  account. There is no password-reset flow yet; plan account changes as a
  data/account migration.

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
- `ION_OF_NOTES_OWNER_PASSWORD` errors mean `.env` is missing or unset.
- Set `RUST_LOG=debug` temporarily for more detail; revert to `info` afterward.

## 9. Multi-arch images

Build for both common server architectures with BuildKit:

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

- [ ] `ION_OF_NOTES_OWNER_PASSWORD` is strong and stored outside git.
- [ ] `.env` is `chmod 600` and not committed.
- [ ] The app is bound to `127.0.0.1` or a private network.
- [ ] A TLS reverse proxy terminates public traffic.
- [ ] Port 8787 is not exposed directly to the internet.
- [ ] Backups run on a schedule and restores are tested.
- [ ] The image is rebuilt for security updates and dependencies are reviewed.
- [ ] Log rotation and host disk monitoring are in place.
- [ ] The container remains non-root with `cap_drop: [ALL]` and
      `no-new-privileges`.
