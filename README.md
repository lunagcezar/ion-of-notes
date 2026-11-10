# ion-of-notes

A self-hosted, local-first workspace for notes, pages and databases. Built
independently in Rust with **eframe/egui** for the desktop window and
**Axum/Tokio/Tower** for the optional HTTP/WebSocket server.

The desktop is local-first and works without a server. The same workspace
contract is also exposed over a loopback API, with accounts, sessions, SQLite
persistence, a versioned change feed, WebSocket CRDT sync and signed webhooks.

Everything is written in English and built spec-first (`spec/`) and test-first
(`cargo test`). The documentation hub is [`docs/index.md`](docs/index.md).

> **AI-assisted and independent project.** This repository was built with
> substantial AI assistance and is not affiliated with Notion Labs, Inc. Review
> and test it before production use. See
> [Provenance and disclaimers](#provenance-and-disclaimers) and the full
> [`DISCLAIMER.md`](DISCLAIMER.md).

## Features

### Desktop workspace

- Page tree, favorites, tabs, breadcrumbs, navigation history and quick search
  (`Ctrl+K`).
- Block editor: headings, paragraphs, bullets, numbered lists, to-dos, quotes,
  callouts, code, dividers, tables, toggles, child pages, column lists and
  unsupported-block placeholders.
- Drag-to-reorder, block handles, slash menu, markdown shortcuts, undo/redo,
  multi-block selection and table row actions.
- Page actions: icons, cover-style callouts, lock, archive, table of contents,
  discussions and inline-comment preferences.
- Library table/board views, filters, sharing dialog, Inbox, version history
  (newest 50 snapshots per page) and export tree.
- Light/dark themes, full-width and small-text modes, zoom, page icons and
  starter templates.
- Localization: English, Brazilian Portuguese and Esperanto for core UI copy.
- Screenshot mode for reproducible visual checks.

### Collaboration and sharing

- Accounts, roles (owner/member/guest), sharing grants and a people directory.
- Versioned sync log and page snapshots.
- Page and block comments with attachments (up to three) and display names.
- Export to a Notion-style markdown tree plus People markdown/CSV.
- Real-time change feed with presence, `?since=` resume and ping/pong.
- Yrs CRDT merge for page title and block text, with client-side awareness
  cursors and selections.

### API and server

- Stable v1 routes and a versioned `/api/v2` contract.
- `Ion-Of-Notes-Version` header, Bearer auth, structured `ApiError`,
  capabilities and request IDs.
- Cursor pagination with `ListResponse`, opaque cursors and `page_size`.
- Rich text, block metadata/children, typed properties, databases, data
  sources and views.
- Comments, users, file uploads and attachments.
- Webhook subscriptions, HMAC-SHA256 signatures, aggregation and retries.
- Rate limits, request-size/block limits and `Retry-After` guidance.
- Axum/Tokio/Tower HTTP adapter with graceful shutdown.
- SQLite persistence (`--database`), Argon2id accounts, opaque sessions and
  JSON→SQLite migration.
- WebSocket `/ws/workspace/{id}` change feed, presence, CRDT sync/update and
  awareness.
- TLS webhook delivery with ureq/rustls, no automatic redirects.
- Shared SQLite rate-limit budgets across server instances using the same file.
- Remote desktop transport: `--server`/`--token` over REST + background
  WebSocket, with reconnect and `?since=` resume.
- Settings → Server login form: server URL, email, masked password,
  connect/disconnect and live local/remote switching.

### AI assistant (optional)

- Ollama integration through the `ollama` CLI in `PATH` or an HTTP service at
  `OLLAMA_HOST` (default `127.0.0.1:11434`).
- Settings → AI assistant can check connectivity, choose a model and ask a
  test question.

## Requirements

- A recent **stable Rust** toolchain (edition 2024).
- A C compiler for bundled SQLite:
  - Windows: Visual Studio Build Tools with the MSVC C++ workload.
  - Linux: `build-essential` or equivalent.
  - macOS: Xcode Command Line Tools.
- PowerShell 7 (`pwsh`) for the optional quality scripts. On Linux/macOS,
  install PowerShell or run the underlying `cargo` commands directly.
- No Node.js, browser engine or web view is required.
- Optional: Ollama for the AI assistant.

## Quick start (desktop)

```sh
git clone <your-repository-url> ion-of-notes
cd ion-of-notes
cargo run --release
```

The window opens at 1322×861 logical points. On first start a neutral demo
workspace is created.

Data locations:

- Windows: `%LOCALAPPDATA%\ion-of-notes\workspace.json` and `collab.json`
- macOS/Linux: `$HOME/ion-of-notes/workspace.json` and `collab.json`

`workspace.json` and `collab.json` are separate documents, so a corrupt
collaboration file cannot destroy pages.

## Run the API server

The server is optional. Without `--database` it uses the JSON documents; with
`--database` it uses SQLite.

```sh
# JSON persistence (same documents as the desktop)
cargo run --release -- --serve 127.0.0.1:8787

# SQLite persistence; an empty database imports the JSON documents once
cargo run --release -- --serve 127.0.0.1:8787 --database data/ion-of-notes.sqlite3
```

Stop with `Ctrl+C`; the Axum adapter shuts down gracefully.

Health check:

```sh
curl http://127.0.0.1:8787/health
```

### First SQLite run and the owner account

The first SQLite run seeds the owner account from
`ION_OF_NOTES_OWNER_PASSWORD`:

```sh
# PowerShell
$env:ION_OF_NOTES_OWNER_PASSWORD = "choose-a-strong-password"
cargo run --release -- --serve 127.0.0.1:8787 --database data/ion-of-notes.sqlite3
```

```bash
# bash
export ION_OF_NOTES_OWNER_PASSWORD="choose-a-strong-password"
cargo run --release -- --serve 127.0.0.1:8787 --database data/ion-of-notes.sqlite3
```

If the variable is not set, the development default `change-me-now` is used and
a warning is printed. The account is seeded **once**; changing the variable
later does not change an existing password (there is no password-reset flow
yet).

### Log in and call v2

```powershell
# PowerShell
$body = @{ email = "owner@workspace.local"; password = $env:ION_OF_NOTES_OWNER_PASSWORD } | ConvertTo-Json
$session = Invoke-RestMethod -Method Post `
  -Uri http://127.0.0.1:8787/api/v2/auth/login `
  -Headers @{ "Ion-Of-Notes-Version" = "2026-11-08"; "Content-Type" = "application/json" } `
  -Body $body

$token = $session.data.token
Invoke-RestMethod -Uri http://127.0.0.1:8787/api/v2/workspace `
  -Headers @{ "Ion-Of-Notes-Version" = "2026-11-08"; "Authorization" = "Bearer $token" }
```

```bash
# bash
TOKEN=$(curl -s http://127.0.0.1:8787/api/v2/auth/login \
  -H 'Ion-Of-Notes-Version: 2026-11-08' \
  -H 'Content-Type: application/json' \
  -d '{"email":"owner@workspace.local","password":"change-me-now"}' \
  | sed -n 's/.*"token":"\([^"]*\)".*/\1/p')

curl -s http://127.0.0.1:8787/api/v2/workspace \
  -H 'Ion-Of-Notes-Version: 2026-11-08' \
  -H "Authorization: Bearer $TOKEN"
```

v1 routes (`/api/workspace`, `/api/pages`, …) stay stable and do not require
the v2 version header. v2 routes require both the version header and a Bearer
token, except login.

### WebSocket endpoint

```text
ws://127.0.0.1:8787/ws/workspace/{workspace_id}?token={session}&since={seq}
```

The `Authorization: Bearer` header is also accepted. Messages are JSON text
frames tagged by `type`:

- Server → client: `ready`, `change`, `presence`, `crdt_sync`, `crdt_update`,
  `awareness`, `pong`, `error`.
- Client → server: `ping`, `crdt_sync`, `crdt_update`, `awareness`.

The socket matches the server workspace (`401`/`404` on failure), replays
changes after `since`, and broadcasts committed changes, CRDT updates and
awareness. See [`docs/api-contract.md`](docs/api-contract.md) for payloads.

### Connect the desktop to a server

The desktop can use the remote HTTP/WebSocket transport instead of the
in-process server:

```sh
cargo run --release -- --server http://127.0.0.1:8787 --token <session-token>
```

`--token` is the session token returned by `/api/v2/auth/login` (or
`owner-token` when the server has no auth store). The desktop loads the
workspace over REST and keeps the change feed, CRDT updates and awareness on a
background WebSocket. Without `--server`, the desktop stays fully local.

You can also connect from **Settings → Server** by entering the server URL,
email and password. Starting with `--server` but no `--token` opens that page
automatically. The password is sent only to the server and is cleared after a
successful login; the session token is not persisted across restarts.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `ION_OF_NOTES_OWNER_PASSWORD` | `change-me-now` | Password for the seeded owner on the first SQLite run |
| `ION_OF_NOTES_SHOW` | unset | Open one pictured UI state for screenshots |
| `ION_OF_NOTES_LOCALE` | `en` | Initial locale (`en`, `pt-BR`, `eo`) |
| `ION_OF_NOTES_OLLAMA_MODEL` | `llama3.2` / detected | Preferred Ollama model |
| `OLLAMA_HOST` | `127.0.0.1:11434` | Ollama service host and port |
| `RUST_LOG` | `info,tower_http=info` | Server tracing filter |

### Preview states

Set `ION_OF_NOTES_SHOW` to capture one pictured state:

```sh
ION_OF_NOTES_SHOW=settings-general cargo run -- --screenshot shots/settings.png
```

Accepted values: `library`, `share`, `inbox`, `settings`, `settings-general`,
`settings-sidebar`, `settings-people`, `settings-offline`, `settings-server`,
`search`, `tasks`,
`newtab`, `ai`, `shortcuts`, `iconpicker`, `pageactions`, `discussions`,
`onboarding`, `templates`.

## Development

```sh
cargo build
# 229 core tests without eframe/egui/wgpu
cargo test --all-targets --no-default-features
# 26 desktop tests with the default-on desktop feature
cargo test --features desktop --test ui --test fidelity --test ui_crdt --test screenshot
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

`cargo test --all-targets` still runs all default-feature tests (254), but the
two passes above avoid linking the window into the core test binaries.

Full gate, including Markdown link/Mermaid/table checks:

```sh
pwsh -File scripts/quality.ps1
```

Screenshot mode renders about 120 frames, writes a PNG and exits:

```sh
cargo run -- --screenshot shots/now.png
```

CI runs formatting, clippy (both feature sets), the core + desktop tests and a
release build. GitHub uses `.github/workflows/ci.yml` on Windows; GitLab uses
`.gitlab-ci.yml` on a Linux Rust image and runs the same
`scripts/quality.ps1` gate.

## Project layout

```text
src/
  main.rs          CLI wiring only (desktop, --screenshot, --serve)
  app.rs           composition root
  model.rs         facade: declarations, re-exports, time helpers
  model/           rich text, blocks, properties, databases, views, files,
                   comments, users, pages, workspace, seed, search, diff,
                   index, settings
  auth.rs          accounts, Argon2id, sessions
  collab.rs        roles, sharing, sync log, versions, realtime DTOs
  collab/          export tree
  store.rs         WorkspaceStore, CollabStore, AuthStore, RateLimitStore ports
  store/sqlite.rs  SQLite adapter and JSON migration
  server.rs        facade: contract DTOs, WorkspaceApi, WorkspaceServer state
  server/          routes (v1/v2/capabilities), handlers, response helpers,
                   Axum/WebSocket, realtime hub, CRDT, webhooks, limits
  client.rs        the UI's only data access
  ui.rs            UI facade (desktop feature)
  ui/              state, commands, rendering, dialogs, client CRDT session
  templates.rs     starter templates
  ollama.rs        AI assistant adapter
  shortcuts.rs     keyboard table
  screenshot.rs    PNG output (desktop feature)
  cli.rs           argument parsing
  i18n.rs          locale switching
tests/             one integration-test crate per module (255 tests)
```

Dependency direction: `ui → client → server → model`, `server → store`; `app`
wires the concrete adapters. `model` and `collab` know nothing about egui,
eframe, JSON files or sockets.

## Documentation

| Read | Why |
| --- | --- |
| [`docs/index.md`](docs/index.md) | Reading order and source-of-truth rules |
| [`docs/architecture.md`](docs/architecture.md) | Layers, modules, request/persistence flows |
| [`docs/contributor-guide.md`](docs/contributor-guide.md) | SDD/TDD workflow and where code belongs |
| [`docs/testing.md`](docs/testing.md) | Test strategy, crates and fakes |
| [`docs/glossary.md`](docs/glossary.md) | Shared vocabulary |
| [`docs/api-contract.md`](docs/api-contract.md) | Implemented v1/v2 routes and WebSocket messages |
| [`docs/public-api-analysis.md`](docs/public-api-analysis.md) | Public API analysis and v2 roadmap |
| [`docs/clean-room.md`](docs/clean-room.md) | Allowed sources, evidence levels, stop conditions |
| [`docs/review-findings.md`](docs/review-findings.md) | Review findings and follow-ups |
| [`spec/README.md`](spec/README.md) | Spec index and status |
| [`AGENTS.md`](AGENTS.md) | Working rules for agents and contributors |

## Current limitations

- Remote mode can log in through Settings → Server, but the token is not
  persisted; users log in again after restart.
- REST requests in remote mode are synchronous, so a slow server briefly blocks
  the UI thread; awareness sent while the socket is disconnected is dropped.
- CRDT covers page title/block text. Block structure, properties and databases
  remain REST-owned.
- The remote-caret overlay uses first-line width, so multi-line positions are
  approximate.
- SQLite is single-node; PostgreSQL and Redis/shared multi-node rate limiting
  are not implemented.
- There is no browser UI, registration, password reset or account-management UI.
- TLS termination, reverse-proxy configuration and deployment manifests are not
  included; run the server behind a trusted proxy for network access.
- Webhooks have no async queue/dead-letter store, and private CA/mTLS setup is
  not configured.
- B3 own-account capture remains blocked by the strict clean-room scope.

## Open source

`ion-of-notes` is open source under the **MIT** license (see [`LICENSE`](LICENSE)).

## Provenance and disclaimers

The full legal and safety notice is in [`DISCLAIMER.md`](DISCLAIMER.md). The
short version:

### Independent implementation

`ion-of-notes` is an independent workspace project. It is **not affiliated
with, endorsed by, sponsored by, or approved by Notion Labs, Inc.** “Notion” is
a trademark of Notion Labs, Inc. Any comparison to Notion or other workspace
products is descriptive only and does not imply compatibility, endorsement or
partnership.

### Clean-room process

The project follows a strict public-behavior-only clean-room process. The 98
`reference/` pictures (the original 29 plus the later build) are the primary
visual source of truth; the 72 `screenshots/` pictures are local later-build
evidence. No original program, file, asset, traffic capture, source map or
decompiled code is read, run or copied. This is an independent reimplementation;
it does **not** claim wire compatibility with any third-party service. See
[`docs/clean-room.md`](docs/clean-room.md) and
[`docs/reverse-engineering-log.md`](docs/reverse-engineering-log.md).

### AI-assisted development

This repository was built with **substantial AI assistance** from a written
request and public documentation. AI-generated code and documentation can
contain mistakes, omissions, insecure patterns or outdated assumptions. The
test suite, `clippy` and documentation checks are necessary but not sufficient
for production use.

Review, audit and test the code before deploying it — especially authentication,
session handling, persistence, networking, webhook delivery and anything exposed
to untrusted input. The software is provided under the MIT license **without
warranty**, and you are responsible for your deployment, data, backups and
security posture.

### No production security guarantee

This is a self-hosted project under active development. Do not expose the server
directly to the public internet without TLS termination, a reverse proxy,
backups and a security review. The development owner password must not be used
in production; set `ION_OF_NOTES_OWNER_PASSWORD` before the first SQLite run.
