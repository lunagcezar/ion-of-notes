# Changelog

All notable changes to this project will be documented in this file.
The format follows "Keep a Changelog" and the SDD cycle described in
`AGENTS.md`: spec → plan → tests (red) → code (green) → changelog → docs.

## [Unreleased]

### Changed - spec-055 remote-mode UX

- Persisted the remote session (`base_url`, `email`, `token`) in
  `remote-session.json`; the password is never written.
- Login/connect and the initial workspace fetch now run on a background thread;
  the UI shows `Connecting...` and polls the result on later frames.
- `--server` restores a saved session on startup; `--server --token` also
  connects in the background. Disconnect and failed restores clear the file.
- Added session round-trip/clear tests; the suite is 261 tests.
- Updated README, architecture, deployment, testing, requirements, glossary,
  spec index and changelog.

### Changed - spec-054 webhook reliability

- Aggregated webhook events are persisted in `CollabState.webhook_queue`, so a
  restart does not lose them.
- The Axum adapter runs a background worker every
  `ION_OF_NOTES_WEBHOOK_INTERVAL_SECS` seconds (default 30) to flush the outbox
  and retry due deliveries without blocking the reactor.
- Added `WebhookDelivery::dead_lettered`, dead-letter listing and requeue
  endpoints; exhausted deliveries stop retrying automatically.
- `/metrics` now exposes webhook pending, dead-letter and delivery counts.
- Added restart, dead-letter, requeue and metrics tests; the suite is 261
  tests.
- Updated the API contract, architecture, deployment, testing, requirements,
  glossary, spec index and changelog.

### Changed - spec-053 supply-chain and tooling baseline

- Added `rust-toolchain.toml` pinning Rust 1.99.1 with `rustfmt` and `clippy`.
- Pinned GitHub Actions by commit SHA and Docker/GitLab base images by digest.
- Added `renovate.json` for grouped Cargo, GitHub Actions and Docker updates.
- Added a `security` job running `cargo audit` in GitHub Actions and GitLab CI.
- Added `.editorconfig` for indentation and line-ending consistency.
- Updated README, contributor guide, architecture, testing, requirements,
  glossary, spec index and changelog. No test-count change (258).

### Changed - spec-052 deployment hardening II

- Added `GET /ready` and `GET /metrics` to the Axum adapter. `/metrics` exposes
  `ion_of_notes_up`, `ion_of_notes_uptime_seconds` and
  `ion_of_notes_http_requests_total`; health/readiness probes do not inflate the
  API request counter.
- Added `ION_OF_NOTES_OWNER_PASSWORD_FILE` and `auth::read_password_file`; the
  environment variable remains the explicit fallback and takes precedence.
- Compose now mounts `secrets/owner_password.txt` as a Docker secret instead of
  putting the password in the environment. Added the example secret file,
  `/secrets/*` ignore rules and `.dockerignore` coverage.
- Added `docker-bake.hcl` for `linux/amd64` + `linux/arm64` builds.
- Added a GitHub Actions container job that builds the image, waits for
  `/ready`, checks `/health` and `/metrics`, and cleans up the container and
  volume.
- Tests: 258 total (232 core + 26 desktop); both clippy passes, fmt, docs and
  the preview screenshots pass.
- Updated README, deployment guide, architecture, testing, requirements,
  glossary, spec index and changelog.

### Changed - spec-049 UI and client modularization

- Split `ui/apply.rs` into command-family modules under `ui/apply/`
  (navigation, library, sharing, blocks, workspace, tabs, ai, settings,
  page actions, CRDT, server) behind the single `apply_commands` entry point.
- Added `CommandFamily` in `ui/commands.rs` as the routing table; the flush and
  lock classifications stay on `Command`.
- Split `ui/dialogs/settings_pages.rs` into one module per settings page and
  moved `section_heading`/`info_row` to `ui/widgets.rs`.
- Split `ui/render/content.rs` into `content/{page,blocks,cursors}.rs` while
  keeping `render_content` as the entry point.
- Extracted `RetryPolicy`/`RetryDecision` into `client/retry.rs` and re-exported
  them from `client.rs`; the public API is unchanged.
- No behavior, protocol, persistence, DTO or visual change. The suite stays at
  255 tests; both clippy passes, fmt, docs and the preview screenshots pass.
- Made the SQLite migration test deterministic by removing stale temporary
  artifacts before use (found by the full gate).
- Updated `AGENTS.md`, README, architecture, review findings, docs hub, spec
  index, ADR-004 and the changelog.

### Changed - spec-051 container hardening and deployment operations

- Compose now binds to `127.0.0.1` by default (`ION_OF_NOTES_BIND` overrides),
  runs with a read-only root, a `/tmp` tmpfs, `cap_drop: [ALL]`,
  `no-new-privileges`, `init`, a 15-second stop grace period, a PID limit and
  rotated `json-file` logs.
- Added `deploy/Caddyfile.example` for a TLS reverse proxy and
  `docs/deployment.md` covering TLS, volume permissions, cold/hot backups,
  restore, secrets, upgrades, health/logs, troubleshooting, multi-arch builds
  and a security checklist.
- Updated `README.md`, `AGENTS.md`, `docs/index.md`, architecture, testing,
  requirements, glossary, spec index and the deployment guide.
- No Rust code or test-count change; the full gate remains 255 tests.

### Added - spec-050 server-only binary and Compose deployment

- Added `src/bin/ion-of-notes-server.rs`, built with
  `--no-default-features`; the image links no eframe/egui/wgpu.
- Extracted `app::serve(addr, database)`; the desktop `--serve` path and the
  server binary share one composition path.
- The Axum adapter now handles `SIGTERM` on Unix as well as Ctrl+C, so
  `docker stop` and `docker compose down` are graceful.
- Added a multi-stage `Dockerfile`, `.dockerignore`, `compose.yml`,
  `.env.example` and `.gitignore` entries for `data/` and `.env`.
- Compose requires `ION_OF_NOTES_OWNER_PASSWORD`, publishes
  `${ION_OF_NOTES_PORT:-8787}`, mounts the `ion-data` volume and inherits the
  `/health` healthcheck.
- Added `default-run = "ion-of-notes"` so `cargo run` keeps opening the desktop
  after the second binary was added.
- Smoke-tested the image: `/health` returns `200`, a created page survives a
  container restart on the same volume, and `docker stop` exits in about 0.5 s.
- The Rust suite stays at 255 tests; fmt, both clippy passes, core + desktop
  tests and `--screenshot` pass.
- Updated `AGENTS.md`, README, architecture, testing, requirements, glossary,
  spec index and the contributor guide.

### Added - GitLab CI pipeline and CI hardening

- Added `.gitlab-ci.yml`: a Linux Rust image installs the desktop build
  dependencies and PowerShell, runs `scripts/quality.ps1` (fmt, both clippy
  passes, core + desktop tests, Markdown checks), then builds the release
  binary. The Cargo registry is cached by `Cargo.lock`.
- The pipeline keeps Cargo's default job count and incremental compilation
  (spec-048); it does not set `-j` or `CARGO_INCREMENTAL`.
- A commented Windows-runner variant is included for self-managed runners.
- GitHub Actions now uses read-only permissions, cancels superseded runs,
  has a 60-minute timeout and builds the release with `--locked`.
- Updated `README.md` and `docs/contributor-guide.md` to describe both the
  GitHub and GitLab pipelines.

### Changed - spec-048 build performance and desktop feature seam

- Added the default-on `desktop` cargo feature. `eframe`, `egui`, `image`,
  `ui` and `screenshot` are absent under `--no-default-features`.
- The binary and `tests/ui.rs`, `tests/fidelity.rs`, `tests/ui_crdt.rs` and
  `tests/screenshot.rs` declare `required-features = ["desktop"]`, so core
  tests skip them.
- `scripts/check-tests.ps1` now runs two passes: 229 core tests without the
  desktop stack, then 26 desktop tests with it (255 total).
- `[profile.test] debug = 0` removes full debug info: test PDBs drop from
  220-270 MB to about 4-15 MB. `[profile.dev] debug = "line-tables-only"`
  keeps file/line backtraces while shrinking the app PDB.
- Incremental compilation and Cargo's default job count stay enabled; the gate
  does not set `-j` or `CARGO_INCREMENTAL`.
- No behavior, protocol, persistence or visual change. Clippy (both feature
  sets), fmt, the two test passes and `--screenshot` pass.
- Updated `AGENTS.md`, `README.md`, `docs/architecture.md`,
  `docs/testing.md`, `docs/requirements.md`, `docs/review-findings.md`,
  ADR-004, `scripts/README.md` and the spec index.

### Changed - spec-047 use-case seam and server sub-state

- Added `src/server/usecases/*`: page, workspace and sharing operations return
  typed results and one `UseCaseError`; they never build an envelope.
- v1 and v2 now adapt the same use cases. The v2 path rewrite into `handle_v1`
  and `map_v1_error` are gone; unknown v2 routes return a structured `404`.
- `route_capability` is public and covered by
  `every_documented_v2_route_declares_a_capability` in `tests/contract.rs`.
- `WorkspaceServer` groups persistence, runtime and webhook state into
  `Persistence`, `Runtime` and `WebhookRuntime`; realtime and CRDT stay as
  single fields.
- No behavior, protocol, persistence or visual change; the suite is 253 tests
  and clippy, fmt, docs and `--screenshot` pass.
- Updated `AGENTS.md`, `README.md`, `docs/architecture.md`,
  `docs/contributor-guide.md`, `docs/index.md`, `docs/testing.md`,
  `docs/review-findings.md` and ADR-004.

### Changed - spec-046 module boundaries

- `src/server.rs` is now a facade over `server/routes/*` (v1 and v2 dispatch,
  capability table), `server/handlers/*` (one module per resource family) and
  `server/response.rs`; the file drops from 3,458 to 545 lines.
- `src/model.rs` is now a facade over `model/{rich_text,block,property,
  database,view,file,comment,user,page,workspace}.rs`; the file drops from
  2,442 to 60 lines and the public `model::*` names are unchanged.
- `Command::forces_flush` and `Command::blocked_when_locked` are the single
  source for those classifications; `apply_commands` consumes them.
- No behavior, protocol, persistence or visual change. The suite stays at 252
  tests; clippy, fmt and `--screenshot` pass.
- Updated `AGENTS.md`, `docs/architecture.md`, `docs/contributor-guide.md`,
  `docs/api-contract.md`, `docs/glossary.md`, `docs/review-findings.md` and
  ADR-004.

### Added - spec-045 server login and settings UI

- Added Settings → Server with server URL, email, masked password, Connect,
  Disconnect and status.
- Added `remote::login` using `/api/v2/auth/login`.
- Connecting swaps `DesktopApi` to the remote transport and reloads workspace,
  tabs, history, pending edits, CRDT and realtime state.
- Disconnecting reloads the local workspace.
- `--server` without `--token` opens the Server page prefilled; `--token`
  without `--server` errors.
- The password is cleared after a successful connect.
- Tests: remote login success/failure; the suite is 252 tests.
- Added `docs/adr-014-server-login-settings-ui.md`.

### Added — spec-044 remote WebSocket client transport

- Added `src/remote.rs`: `RemoteApi` with ureq REST and a background
  `tokio-tungstenite` socket that reconnects with `?since=`.
- Added REST CRDT routes `POST /api/v2/crdt/sync` and
  `POST /api/v2/crdt/update` with base64 Yrs payloads.
- Added `DesktopApi::{Local, Remote}`, `build_remote_desktop` and
  `NotionApp::with_client`.
- Added `--server` and `--token`; remote mode requires both.
- Remote changes and awareness use the same broadcast receiver as local mode.
- Tests: 2 in `tests/remote.rs` plus CLI parsing; the suite is 251 tests.
- Added `docs/adr-013-remote-websocket-client.md`.

### Added — spec-043 client-side Yrs and awareness

- Added `src/ui/crdt.rs`: client `CrdtSession` with local text diff, remote
  apply, state vector and page projection.
- Added default CRDT/realtime/awareness methods to `WorkspaceApi`; the
  in-process `WorkspaceServer` overrides them and `Client` exposes typed
  wrappers.
- Added `Awareness` (cursor/selection) storage and broadcast to the realtime
  hub; WebSocket `awareness` messages.
- The editor sends `CrdtTextChanged`/`CrdtCursor`; the app drains realtime
  events, projects merged text and filters its own awareness.
- Added a remote caret/name/selection overlay in the block editor.
- Local CRDT edits mark committed block text so the REST debounce does not reset
  the server document.
- Tests: 4 in `tests/ui_crdt.rs` plus a hub-awareness test; the suite is 248
  tests. Added `docs/adr-012-client-yrs-and-awareness.md`.

### Added — spec-042 Yrs CRDT text merge

- Added `src/server/crdt.rs`: one Yrs `Doc` per page with title and
  `block:<id>` text, state-vector sync, update merge, projection and reset.
- `WorkspaceServer::crdt_sync` / `apply_crdt_update` merge text, project it into
  the workspace snapshot, record a sync change and persist.
- WebSocket messages now include base64 `crdt_sync` and `crdt_update`; merged
  updates broadcast to subscribers.
- REST writes reset the page CRDT document so structure stays REST-owned.
- Added `tests/crdt.rs`: 3 tests for convergence, projection and WebSocket sync.
- Added `docs/adr-011-yrs-crdt-merge.md`; the suite is 243 tests.

### Added — spec-041 TLS webhooks and shared rate limits

- Added `src/server/webhook_sender.rs`: `WebhookSender` port and
  `UreqWebhookSender` with rustls, a 5-second timeout, no redirects and
  non-2xx statuses as values.
- Removed the hand-written loopback TCP POST; the server now uses an injectable
  sender (tests use a recording sender).
- Added `RateLimitStore`/`RateLimitHit` to the store ports and a transactional
  SQLite `rate_limits` table.
- Added `RateLimitPolicy` and `SqliteRateLimiter`; `--database` now shares the
  workspace budget across server instances using the same SQLite file.
- Added `tests/shared_limits.rs` (2 tests) and an injected-sender webhook test;
  the suite is 240 tests.
- Added `docs/adr-010-webhook-delivery-and-shared-limits.md`.

### Added — spec-040 real-time WebSocket change feed

- Added `src/server/realtime.rs`: `RealtimeHub`, `ServerMessage`,
  `ClientMessage` and `PresenceMember`.
- `WorkspaceServer::write` publishes every committed change after a successful
  persist; `WorkspaceServer::realtime()` exposes the hub.
- Added `GET /ws/workspace/{id}` with Bearer/query auth, workspace matching,
  `?since=` backlog replay, `ready`/`change`/`presence`/`pong` messages and
  ping/pong.
- Updated `RealtimeContract` with the socket URL, resume cursor, message names
  and staged CRDT note.
- Added `tests/realtime.rs`: 6 tests; the suite is 237 tests.
- Added `docs/adr-009-realtime-transport.md`.
- Full Yrs/Automerge merge remains staged.

### Added — spec-039 sessions, Argon2id and SQLite storage

- Added `src/auth.rs`: accounts, Argon2id password hashes, opaque session
  tokens (SHA-256 at rest), `AuthStore` port plus null/in-memory adapters.
- Added `src/store/sqlite.rs`: `SqliteStore` implementing `WorkspaceStore`,
  `CollabStore` and `AuthStore`, with WAL mode and JSON-document storage.
- `--database <path>` selects SQLite for `--serve`; an empty database migrates
  the existing JSON workspace/collab documents once.
- Added `POST /api/v2/auth/login`, `POST /api/v2/auth/logout` and
  `GET /api/v2/auth/session`; sessions survive server restarts.
- The owner account is seeded from `ION_OF_NOTES_OWNER_PASSWORD` (with a
  documented development default).
- Added `docs/adr-008-sessions-and-sqlite.md`.
- Tests: 5 in `tests/auth.rs`, 3 in `tests/sqlite_store.rs` and one CLI test;
  the suite is 231 tests. Docs updated.

### Added — spec-038 Axum web server and contract suite

- Added `src/server/web.rs`: an Axum/Tokio/Tower adapter for `--serve` behind
  the existing `WorkspaceApi` seam, with `spawn_blocking`, tracing, a 2 MB
  transport body cap and graceful Ctrl+C shutdown.
- `--serve` now uses Axum; the hand-written loopback listener remains for
  parser/formatter tests and as a fallback.
- Responses carry `X-Request-Id` and `Retry-After`; `Content-Type` is set by
  the adapter.
- Added `tests/web.rs`: 9 contract tests for health, v1 pages, v2
  version/auth, request IDs, `Retry-After`, 501 KB validation, query strings,
  pagination, webhooks and structured 404s.
- Added `docs/adr-007-production-web-server.md`.
- B5 from spec-016 is complete; the suite is 221 tests. Docs updated.

### Changed — project rename to ion-of-notes

- Renamed the crate/package and binary from `not-notion` to `ion-of-notes`
  (Rust crate `ion_of_notes`).
- Renamed the preview/environment hooks to `ION_OF_NOTES_SHOW`,
  `ION_OF_NOTES_LOCALE` and `ION_OF_NOTES_OLLAMA_MODEL`.
- Renamed the v2 header to `Ion-Of-Notes-Version` and the webhook signature
  header to `X-Ion-Of-Notes-Signature`; API paths, payloads and behavior are
  unchanged.
- Updated the window title, locale `app_name`, data directory, URL scheme,
  webhook `integration_id`, tests, scripts and all documentation.

### Changed — spec-016 A9 settings row filtering

- Added a per-page `SettingsRow` table in `ui/settings_search.rs`; the table is
  the single source of truth for matching and rendering.
- Settings search now filters rows inside the open section, keeps a section in
  the rail when one of its rows matches, and shows a section-level no-match
  message.
- Tests: 2 in `tests/ui.rs` for row filtering, uniqueness and section matching;
  the suite is 212 tests. Docs updated.

### Added — spec-037-limits-and-operations

- Added `src/server/limits.rs`: `Plan`, `RequestBudget`, request-size
  validation and the Free multi-member 1000-block policy.
- Added `src/server/rate_limit.rs`: per-connection and shared workspace
  fixed-window budgets with `Retry-After` seconds and `rate_limit_reason`.
- `/api/v2` returns `429 rate_limited`, `529 service_overload`,
  `503 service_unavailable` (with retry guidance) and `409 conflict_error`
  (`If-Match`); the HTTP adapter emits `Retry-After`.
- Enforced 500 KB, 1000 block elements, 2000-character rich text/URL,
  1000-character equation, 100 array items and 200-character email/phone
  limits before any mutation.
- Added `RetryPolicy`/`RetryDecision` to the client: honours `Retry-After`,
  uses capped exponential backoff plus caller-supplied jitter and only retries
  the documented statuses.
- Tests: 14 in `tests/limits.rs` plus one retry-policy test; the suite is
  210 tests at the time (212 after spec-016 A9). Docs updated.

### Added — spec-036-webhooks-and-event-delivery

- Added `src/webhooks.rs`: subscriptions, status, all documented page,
  database, data-source, comment, file-upload and view event types, shared
  event payloads, HMAC-SHA256 signatures and aggregation.
- Subscriptions and delivery records persist in `collab.json`.
- Added v2 webhook endpoints: create/list/delete/verify and delivery
  list/retry; loopback HTTP requires an explicit development override.
- Wired page, comment, file-upload and view mutations to emit events.
- Added client methods for webhooks and deliveries.
- Added `hmac` and `sha2` dependencies.
- Tests: 9 in `tests/webhooks.rs`; the suite is 195 tests. Docs updated.

### Added — spec-035-comments-users-and-files

- Added rich comment DTOs (`CommentDto`, `CommentInput`,
  `CommentAttachment`, display names) with page/block parents, discussions and
  up to three validated attachments.
- Added `UserDto` person/bot objects and capability-filtered profile fields.
- Added `FileObject`, `FileUpload`, `FileUploadInput` and `UploadStatus` with
  the create/send/complete lifecycle.
- `Workspace` persists v2 comments and file uploads; the existing page-comment
  UI model is unchanged.
- Added v2 endpoints for comments, users and file uploads, plus client methods.
- Tests: 8 in `tests/comments.rs`; the suite is 186 tests. Docs updated.

### Added — spec-034-views-and-database-layouts

- Added `ViewType`, `Sort`, `SortDirection`, `Filter`, `ViewConfig`, `View`,
  `ViewInput`, `ViewQueryInput`, `ViewGroup` and `ViewQueryResult`.
- `Workspace` persists views; `ensure_views` creates one default table view
  per data source.
- Added v2 view endpoints for list/create, retrieve/update/delete and query.
- View queries filter and sort data-source rows, use cursor pagination and
  return grouped pages for board/calendar configurations.
- Added client methods for list/create/get/update/delete/query views.
- Tests: 10 in `tests/views.rs`; the suite is 178 tests. Docs updated.

### Added — spec-033-page-properties-and-data-sources

- Added property schemas (`Property`, `PropertyType`, `PropertyConfig`,
  select/status options) and typed page values (`PageProperty`,
  `PropertyValue`, dates, options, relations, formula/rollup results).
- `Page` now carries `data_source_id` and typed `properties`.
- Added `Database`, `DataSource`, `DataSourceRef`, `DatabaseInput`,
  `DataSourceInput` and `PagePropertiesInput`; `Workspace` persists databases
  and data sources.
- `Workspace::ensure_data_sources` migrates legacy `TableRow` pages into a
  default database/data source with row pages and typed title/status/assignee/
  due values.
- Added v2 endpoints for databases, data sources, data-source queries, page
  property writes and paginated property items.
- Added client methods for databases, data sources, queries and property
  items.
- Tests: 9 in `tests/properties.rs`; the suite is 168 tests. Docs updated.

### Tooling — reusable check scripts

- Added `scripts/check-links.ps1`, `check-mermaid.ps1`, `check-tables.ps1`,
  `check-docs.ps1`, `check-tests.ps1` and `quality.ps1`.
- `quality.ps1` runs `cargo fmt --check`, clippy, tests and the Markdown checks
  in one command, which makes command approval short and repeatable.
- Added `scripts/README.md`; updated `AGENTS.md`, `README.md`,
  `docs/testing.md` and `docs/contributor-guide.md`.

### Added — spec-032-rich-text-and-blocks

- Added rich text objects (`text`, `mention`, `equation`) with annotations,
  `plain_text` and `href`, plus `TextContent`, `TextLink`, `Mention` and
  `Equation`.
- `Block` now carries `object`, parent-block metadata, timestamps, users,
  `in_trash`, nested `children`, rich text, table metadata/cells, child-page
  title and an explicit unsupported type.
- Added `BlockKind::Heading4`, `Table`, `TableRow`, `Toggle`, `ChildPage`,
  `ColumnList`, `Column` and `Unsupported`; the UI/export fall back safely.
- Legacy `Block.text` is migrated to rich text on load; the egui editor keeps
  using `text`, and v2 rich-text writes sync it.
- Added `BlockDto`, `BlockInput`, `BlockChildrenRequest`, `ParentDto` and
  `UserRef`.
- Added `/api/v2/blocks/{id}`, `GET`/`PATCH /blocks/{id}/children`,
  `PATCH /blocks/{id}` and `DELETE /blocks/{id}`; children use cursor
  pagination.
- Added client block methods for retrieve, list, append, update and trash.
- Tests: 12 in `tests/blocks.rs`; the suite is 159 tests. Docs updated.

### Added — spec-031-pagination

- Added `ListResponse<T>` (`object:"list"`, `type`, `results`, `has_more`,
  `next_cursor`) and a pure `src/server/pagination.rs` cursor codec.
- `/api/v2` list routes for pages, search, members, shares, versions and sync
  now accept `page_size` (1–100, default 100) and `start_cursor`.
- Cursors are opaque hex payloads bound to an endpoint/query sort key; a
  malformed or wrong-list cursor returns `400 invalid_cursor`.
- Added `Client` `*_page_v2` methods and cursor-following helpers for pages,
  search, members, shares, sync and versions, with a termination guard.
- Tests: 11 in `tests/pagination.rs`; the suite is 147 tests. Docs updated.

### Added — spec-030-api-v2-foundation

- Added `/api/v2` with a required `Ion-Of-Notes-Version: 2026-11-08` header,
  Bearer-token authentication and capability checks for content read, update
  and insert.
- Added `ApiError` with `object`, `status`, `code`, `message`, `request_id` and
  optional `additional_data`; v2 errors use stable codes such as
  `missing_version`, `unsupported_version`, `invalid_json`,
  `validation_error`, `unauthorized` and `restricted_resource`.
- Added request IDs to `ApiResponse`, the `X-Request-Id` HTTP header and an
  in-memory request log.
- Added `Capability`, `Connection`, `RequestLogEntry`, `API_VERSION` and
  header parsing; the local demo owner token is `owner-token`.
- Added `Client::with_token` and typed `*_v2` methods that decode structured
  errors as `ClientError::Api`.
- v1 routes and envelopes are unchanged.
- Tests: 17 contract tests in `tests/contract.rs` plus 4 v2 client tests; the
  suite is 136 tests. Docs updated (`api-contract`, requirements, architecture,
  testing, glossary, review findings, index, AGENTS, README, spec index).

### Docs — public Notion API analysis and v2 roadmap (B4)

- Reviewed the public Notion developer documentation (intro, pagination,
  capabilities, webhooks, delivery, request/workspace limits, status codes,
  versioning, changes-by-version, block, rich text, page, page properties,
  property items, database, data source, property object, view, comments,
  attachments, display name, file object/upload, user, parent, emoji/icon,
  unfurl). No capture or private source was used.
- Added `docs/public-api-analysis.md`: conventions, errors, pagination,
  capabilities, blocks/rich text, pages/properties, data sources/views,
  comments/users/files, webhooks, limits, comparison, decisions and roadmap.
- Added `docs/adr-006-public-api-alignment.md`: adopt compatible patterns
  behind our own `/api/v2`; keep v1 stable; do not clone Notion.
- Added specs 030–037: v2 foundation, pagination, rich text/blocks, page
  properties/data sources, views, comments/users/files, webhooks, limits.
- Updated `docs/api-observations.md` with the **Observed (public docs)**
  evidence level and the B4 summary; updated `docs/api-contract.md`,
  `docs/requirements.md`, `docs/data-model.md`, `docs/architecture.md`,
  `docs/glossary.md`, `docs/testing.md`, `docs/clean-room.md`,
  `docs/reverse-engineering-log.md`, `docs/index.md`, `docs/review-findings.md`,
  `AGENTS.md` and `spec/README.md`.
- No production code changed. Verification: 115 tests green, clippy and
  `cargo fmt --check` clean.

### Docs — spec-029-documentation-and-contributor-guide

- Added the documentation hub (`docs/index.md`) with reading order, a document
  catalog and source-of-truth rules.
- Added `docs/architecture.md` (system context, layers, module map, request and
  persistence flows, design principles) and `docs/contributor-guide.md`
  (SDD/TDD workflow, where code belongs, pitfalls, commit checklist), both with
  Mermaid diagrams.
- Added `docs/testing.md` (17 test crates, 115 tests, fakes, determinism,
  visual checks), `docs/glossary.md`, `docs/review-findings.md`,
  `docs/api-contract.md` (implemented loopback API vs decided `/api/v1`), and
  `docs/clean-room.md` (allowed sources, evidence levels, B1–B8, stop
  conditions).
- Added `spec/README.md` (spec index, lifecycle, open work) and this spec.
- Fixed documentation drift: NFR-10..15 moved to §2; FR-02/57/68/71 statuses
  corrected; ADR-001..005 updated with current implementation status; spec-002,
  008, 011, 014 and 016 annotated; unknowns, API observations and the reverse-
  engineering log updated.
- Corrected the visual evidence count: `reference/` now has **98** pictures
  (original 29 + later build) and `screenshots/` has **72** local pictures.
  `docs/visual-reference-inventory.md` now covers both sets.
- No production code or behavior changed. Verification: 115 tests green,
  clippy and `cargo fmt --check` clean.

### Added — spec-027-onboarding-and-templates

- Added `WorkspaceSettings.onboarding_completed` and `workspace_purpose`.
- Added a pure starter-template catalog (`work-project`, `personal-notes`,
  `school-course`, `meeting-notes`, `blank`) with block/table-row blueprints.
- Added the “What is this space for?” onboarding modal with For work /
  For personal life / For school and Skip for now; choosing a purpose applies
  the matching template and persists the choice.
- Added the Template library window, reachable from Library → More → Templates
  and Settings → General → Get started.
- Added `ION_OF_NOTES_SHOW=onboarding|templates` previews.
- Red: template catalog/page-builder and onboarding-purpose tests were written
  first.
- Docs: added `spec/spec-027-onboarding-and-templates.md`; updated
  requirements, data model, README and `AGENTS.md`.

### Changed — spec-028-i18n-coverage

- Added `i18n::t(key)` for dynamic translation keys and expanded the `en`,
  `pt-BR` and `eo` key tables with matching key sets.
- Translated the sidebar, Share dialog, Library, page actions, comments and
  Settings labels/status messages; Library tab state stays keyed by stable
  English names while the display label is translated.
- Count-bearing status messages use `%{count}` placeholders.
- Added `tests/i18n.rs`: key completeness across locales, locale switching and
  unknown-locale fallback to English.
- Docs: added `spec/spec-028-i18n-coverage.md`; updated requirements and
  `AGENTS.md`.

### Added — spec-026-page-actions-and-comments

- Page actions now include Copy page contents (markdown to clipboard), a
  rendered Table of contents, Page discussions, Inline comments toggle,
  Archive/restore and status feedback for placeholder actions.
- `Page` persists `show_toc`, `discussions`, `inline_comments`, `archived` and
  `comments`; `Comment` is a serde model type.
- Page discussions support listing, adding and deleting comments.
- Archived pages are hidden from the sidebar, Favorites, search, Recents,
  Shared and Private; the Library More menu has an Archive view.
- Added `ION_OF_NOTES_SHOW=discussions` for the TOC/comments preview.
- Red: page flags/comments round-trip, heading filtering, archived
  favorites/search and Library archive tests were written first.
- Docs: added `spec/spec-026-page-actions-and-comments.md`; updated
  requirements, data model, README and `AGENTS.md`.

### Added — spec-025-settings-pages

- Added persisted `WorkspaceSettings` on `Workspace`: workspace id/icon,
  landing page, allowed domains, sidebar visibility toggles, other-apps,
  analytics, offline automatic downloads and People toggles.
- General page: editable workspace name, icon, landing page, allowed email
  domains, invite members and export buttons.
- Sidebar page: working visibility toggles, other-apps, exports and analytics.
- People page: member directory, directory/activity/hover toggles, a disabled
  danger-zone action and the workspace ID.
- Offline page: automatic downloads, searchable offline pages and last-edited
  times.
- Workspace-level settings changes are detected by `PageDiff.settings_changed`
  and use the full `PUT /api/workspace` fallback.
- Added `ION_OF_NOTES_SHOW=settings-general|settings-sidebar|settings-people|settings-offline`
  previews.
- Red: `WorkspaceSettings` round-trip and settings-diff tests were written
  first.
- Docs: added `spec/spec-025-settings-pages.md`; updated requirements, data
  model, README and `AGENTS.md`.

### Changed — spec-024-library-views

- Library tabs are functional: Favorites filters favorite pages, Shared shows
  pages with a page-specific share, Private shows pages without one, and
  Recents shows the workspace order. The general `"*"` share does not mark
  every page as shared.
- Replaced the `More` tab with a menu offering AI Meeting Notes, Agents,
  Templates and Trash placeholder views.
- Library rows now show the owner from the member directory, the workspace
  name as Source, and the page's real `updated_at` as Last edited time.
- `OpenLibrary` refreshes shares/members; `ION_OF_NOTES_SHOW=library` populates
  the preview.
- Added pure `ui/library_filter.rs` and its tests.
- Docs: added `spec/spec-024-library-views.md`; updated requirements, README
  and `AGENTS.md`.

### Added — spec-023-add-members-sharing-dialog

- The Share popover is functional: it loads the member directory and general
  access when opened, supports comma-separated emails, Guest/Member roles and
  an optional message, sends invites through `Client::invite_member`, refreshes
  the list and shows inline status/errors.
- General access cycles Full → Can edit → Can view → No access → Full and is
  persisted with `Client::set_share`; Copy link copies the page link.
- Added pure `ui/invite.rs` helpers (`parse_invite_emails`, `is_valid_email`,
  `member_name_from_email`) and `collab::general_access`.
- Added `ION_OF_NOTES_SHOW=share` population for screenshot previews.
- Red: new invite-helper tests in `tests/ui.rs` and the general-access test in
  `tests/collab.rs` were written first and observed failing.
- Docs: added `spec/spec-023-add-members-sharing-dialog.md`; updated
  requirements, README and `AGENTS.md`.

### Changed — spec-022-workspace-index-and-version-retention

- Added a private `WorkspaceIndex` (`model/index.rs`) mapping page ids to
  positions and parents to child ids. `page`, `page_mut`, `children` and
  `breadcrumbs` now use it, with a safe linear fallback after direct `pages`
  mutations.
- The index is skipped by serde, rebuilt after deserialization and after
  `add_page`/`remove_page`, and ignored by `Workspace` equality; the JSON shape
  is unchanged.
- Added `MAX_VERSIONS_PER_PAGE` (50). `CollabState::record_version` prunes the
  oldest snapshots for the same page and leaves other pages untouched.
- Red: new index tests in `tests/model.rs` and the retention test in
  `tests/collab.rs` were written first and observed failing.
- Docs: added `spec/spec-022-workspace-index-and-version-retention.md`; updated
  requirements, data model, README and `AGENTS.md`.

### Changed — spec-021-quality-and-concurrency-baseline

- Added `ClientError::Encode` and `client::encode_body`; every mutation now
  uses the checked encoder instead of `unwrap_or_default()`, so a serialization
  failure cannot send an empty request body.
- `WorkspaceStore` and `CollabStore` are now `Send + Sync`, allowing the server
  to be shared across connection threads.
- Added `server::serve_on`: requests are read on the connection thread, the
  server state is locked only for `handle`, and each connection gets its own
  worker. `serve_forever` delegates to it.
- Red: `tests/client.rs` encode-failure and `tests/server.rs` two-connection
  concurrency tests were written first and observed failing.
- `cargo fmt --check` is now clean; the formatting-only change is committed
  separately.
- Added `.github/workflows/ci.yml` for Windows: formatting, clippy, tests and a
  release build.
- Docs: added `spec/spec-021-quality-and-concurrency-baseline.md`; updated
  requirements and `AGENTS.md`.

### Changed — spec-020-per-page-writes-and-debounce

- Single-page edits now persist with `PATCH /api/pages/{id}` instead of a
  whole-workspace `PUT`. The full snapshot remains the fallback for added,
  removed or reordered pages.
- Added `model::PageDiff` (changed/added/removed/reordered pages) and
  `ui::edit_queue` (`CommitPlan`, `PendingEdits`, `execute_plan`).
- The UI keeps the last successfully committed workspace and debounces writes
  for 250 ms. The debounce timer resets only when the diff actually changes, so
  a redraw cannot postpone the flush forever.
- Pending edits are force-flushed on page/tab switch, Library/Settings,
  reload, new/delete page, reset, version history, restore and window close;
  `on_exit` also flushes when the OS closes the window.
- Red: new `tests/model.rs` diff tests and `tests/ui.rs` planning/debounce/
  execute-plan tests were written first and observed failing.
- Docs: added `spec/spec-020-per-page-writes-and-debounce.md`; updated
  requirements, README and `AGENTS.md`.

### Added — spec-019-collaboration-persistence

- Collaboration state is now durable. Added a `CollabStore` port with
  `JsonCollabStore` and `NoCollabStore` adapters, plus `collab_path`,
  `save_collab`, `load_collab` and `load_collab_or_seed` in `src/store.rs`.
- `workspace.json` and `collab.json` are separate documents, so a corrupt
  collaboration file cannot destroy pages. Missing/corrupt collaboration files
  fall back to the documented seed.
- `WorkspaceServer` now owns a collaboration store. `with_stores` injects both
  adapters and `with_paths` persists both documents; `new` and `with_path`
  keep their workspace-only behavior.
- `POST /api/members` and `PUT /api/shares` persist collaboration state and
  return `500` when the collaboration store fails.
- The composition root loads/seeds both documents and exposes
  `build_server_at_paths` / `build_desktop_at_paths`.
- Red: six tests in `tests/collab_store.rs` were written first and observed
  failing (missing `CollabStore`, `JsonCollabStore`, `NoCollabStore`,
  `save_collab`, `load_collab`, `load_collab_or_seed`, `with_paths`).
- Docs: added `spec/spec-019-collaboration-persistence.md`; updated
  requirements, data model, README and `AGENTS.md`.

### Changed — spec-018-domain-and-infrastructure-modularization

- Pure refactor of the three remaining multi-responsibility non-UI files,
  split by responsibility (ADR-004), with no behavior or API change:
  - `model.rs` 532 → 356 lines: entities and tree behavior stay; the demo
    workspace moves to `model/seed.rs` (133) and ranked search to
    `model/search.rs` (66).
  - `collab.rs` 532 → 381 lines: DTOs and `CollabState` stay; the markdown/CSV
    export tree moves to `collab/export.rs` (149).
  - `server.rs` 518 → 364 lines: routes and `WorkspaceServer` stay; the
    loopback HTTP adapter moves to `server/http.rs` (114) and URL encoding to
    `server/url.rs` (44).
- `lib.rs` and the test imports keep working through the module facades.
- `AGENTS.md` and `README.md` document the new tree.
- `cargo build`, `cargo test --all-targets` (85 tests), clippy, rustfmt and the
  default screenshot are unchanged and green.
- Added `spec/spec-018-domain-and-infrastructure-modularization.md`.

### Changed — spec-017-ui-modularization

- Pure refactor of the three multi-responsibility UI files, split by
  responsibility (ADR-004), with no behavior or visual change:
  - `ui/app.rs` 1508 → 389 lines: state shell + `new()` + `eframe::App`;
    command application moved to `ui/apply.rs`; command shapes to
    `ui/commands.rs`; view snapshot to `ui/view.rs`; key translation to
    `ui/input.rs`.
  - `ui/dialogs.rs` 2029 → 21-line facade: settings, settings pages, sharing,
    page actions/version history, menus, shortcut sheet and search/new-tab
    windows now live in cohesive `ui/dialogs/*` modules; shared popover/menu
    widgets moved to `ui/widgets.rs`.
  - `ui/render.rs` 1335 → 16-line facade: chrome, sidebar, content, Library and
    inline database now live in `ui/render/*`.
- `ui.rs` stays the facade; `AGENTS.md` and `README.md` document the new tree.
- `cargo build`, `cargo test --all-targets` (85 tests), clippy and the default
  screenshot are unchanged and green.
- Added `spec/spec-017-ui-modularization.md`.

### Planned — spec-016-architecture-follow-up-and-observation

- Added `spec/spec-016-architecture-follow-up-and-observation.md`: phased
  follow-up work for collaboration persistence, per-page writes and debounce,
  a workspace index, version retention, the `ui/app.rs` split, localization
  coverage, explicit client encode errors, a concurrent loopback listener,
  settings row filtering, and a `cargo fmt --check` baseline.
- Added `docs/adr-005-clean-room-server-observation.md`: public documentation,
  own-account HAR captures, exports and a derived mock contract are the primary
  discovery sources; Wireshark/mitmproxy are only for traffic we control, with
  redaction and a clean-room log. This is a plan, not an implementation yet.
- **B1/B2 done under the strict scope:** the person chose **public behavior
  only** (no HAR/proxy/Wireshark). `docs/api-observations.md` now holds the
  first inventory, marked UI-observed / inferred / decided / unknown, and
  `docs/reverse-engineering-log.md` records the decision. B3 is intentionally
  blocked unless the scope changes.
- **Visual reference inventory:** reviewed the 72 pictures in `screenshots/`
  (a later UI build) and recorded their surfaces in
  `docs/visual-reference-inventory.md`: new-tab switcher, Add members, page
  actions, Library tabs, onboarding, Settings General/Sidebar/People/Language/
  Offline, template library, AI/agent surfaces and slash/icon pickers. The
  inventory is UI-observed evidence only; no Portuguese text, branding or
  assets were copied.

### Added / Changed — spec-015-hardening-and-performance

- **Settings search:** the Settings rail now actually filters by the query
  (title or description), jumps to the first matching section, and shows a
  clear "No settings match" message. The matching rule is a pure helper in
  `ui/settings_search.rs` (spec-014 item 4 completed).
- **Version history:** the page actions "Version history" entry is functional.
  `Client::page_versions` reads the existing
  `GET /api/pages/{id}/versions` route, a new window lists snapshots newest
  first, and "Restore this version" writes the chosen snapshot back through
  `Client::update_page`. New `Command`s keep the UI on the same client-bound
  path.
- **Search performance:** `ui/search_cache.rs` caches the last query and
  workspace version, so an unchanged frame no longer asks the server for search
  results. Results are stored in an `Arc` and cloned cheaply into `ViewState`.
- **Persistence reliability:** `store::save` writes to a sibling temporary file
  and then moves it into place, so an interrupted save cannot truncate
  `workspace.json`. `WorkspaceServer::write` now returns the storage error and
  mutation routes answer `500` instead of silently claiming success.
- **Performance build:** added a thin-LTO release profile.
- **Quality:** `cargo clippy --all-targets -- -D warnings` is clean; the
  pre-existing `map_or(false, …)`, collapsible-`if`, sort and `io::Error`
  lints were fixed.
- Red: new tests in `tests/ui.rs` (settings matcher, search cache),
  `tests/client.rs` (version order), `tests/store.rs` (atomic save) and
  `tests/store_port.rs` (failing store → 500) were written first.
- Docs: added `spec/spec-015-hardening-and-performance.md`, updated
  `docs/requirements.md`, `docs/unknowns.md`, `README.md` and the module tree
  in `AGENTS.md`.

### Added — spec-014-bug-fixes-and-improvements

- **Editor:** Backspace in an empty text block deletes the block (keeping at least one). Block row layout is kept stable across hover to avoid focus loss while typing; text edits use persistent IDs. Locked pages make content read-only and block structural edits.
- **Drag & reorder:** The six-dot block handle uses click-and-drag with drop targeting; hover/drag state is tracked for reorder.
- **Modals:** Search/new-tab, settings, share, inbox, shortcuts, page actions, and icon picker close when clicking outside their window.
- **Icon picker:** Emoji tab now includes actual emoji glyphs (with Segoe UI Emoji fallback) alongside an Icons tab and Upload; icon choices render consistently.
- **Sidebar / chrome:** Added sidebar inner padding, interactive workspace/account row dropdown (workspace settings, invite members, add account, switch/create workspace), clickable grid menu (Library, Skills, Trash, Settings), functional "Customize sidebar" entry opening settings, and a compact app menu (File/Edit/View/History/Window/Help) replacing the pin-like quick-note button.
- **Library:** Tabs (Recents, Favorites, Shared, Private, More) are functional and filter the page list.
- **Settings:** Search settings field is interactive; added Workspace settings, Sidebar customization, Offline pages, and People sections. Language & time shows a working language selector (cycles supported locales) and time format row.
- **Internationalization:** Added `rust-i18n` with `en`, `pt-BR`, and `eo` locale files and an `i18n` module; locale can be changed from Preferences.
- **Page state:** `Page` gained `locked` and `available_offline` fields (serde defaults) reflected in UI toggles and persisted.
- **Version history (server):** Added `PageVersion` snapshots in `CollabState`, `/api/pages/{id}/versions`, and snapshot recording on page create/update and workspace replace.
- **Docs:** Added `spec/spec-014-bug-fixes-and-improvements.md`.

### Added — spec-001-project-foundation

- Created `AGENTS.md` with the architecture (client + server), the Rust/eframe
  stack, the SDD workflow, the TDD discipline, the API contract, and commands.
- Created `docs/requirements.md` (functional FR-01..FR-33 and non-functional
  NFR-01..NFR-07) and `docs/data-model.md` (Workspace, Page, Block, TableRow,
  invariants).
- Created `spec/spec-001` .. `spec/spec-006` with Given/When/Then acceptance
  criteria and test scenarios written **before** implementation.
- Created the Cargo project in `app/` and confirmed `cargo build` succeeds.

### Added — spec-002-domain-model

- Red: ten tests in `src/model.rs` were written first and observed failing.
- Green: `Workspace`, `Page`, `Block`, `BlockKind`, `TableRow`, tree helpers,
  ranked case-insensitive search, breadcrumbs, JSON round-trip, version bump.

### Added — spec-003-workspace-server

- Red: twelve tests in `src/server.rs` were written first.
- Green: `WorkspaceApi` trait, `WorkspaceServer` with REST-shaped routes,
  JSON envelopes, persistence hooks, HTTP parsing/formatting, and the
  `--serve` loopback listener.

### Added — spec-004-client-layer

- Red: eight tests in `src/client.rs` were written first (including a stub
  API for error paths).
- Green: generic `Client<A: WorkspaceApi>` with typed methods and
  `ClientError` mapping for HTTP and decode failures.

### Added — spec-005-desktop-ui

- Red: palette and menu tests in `src/ui.rs` were written first.
- Green: eframe/egui window with sidebar, breadcrumb top bar, editable title,
  ten block renderers, slash-command menu, inline table and board views,
  quick search, settings/theme dialog, expandable page tree.
- All UI state changes go through `Client`.

### Added — spec-006-screenshot-mode

- Red: PNG and CLI tests were written first.
- Green: `--screenshot <file.png>` draws ~120 frames, captures the window via
  `egui::Context::request_screenshot`, writes a PNG, and exits.
- Verified by running `--screenshot app/shots/now.png` and inspecting the PNG.

### Changed — visual polish after the first screenshot review

- Reworked the inline table so column widths are fixed; full row names are now
  visible instead of being clipped, with striped rows and colored status chips.
- Status chips are clickable and cycle through `Not started` → `In progress` →
  `Done`, and the slash-command menu gained a `Delete block` action
  (FR-14).
- Verified the Tasks page and Board view by running `--screenshot` with the
  Tasks page set as the landing page; removed the temporary data file so the
  app re-seeds the normal demo workspace on the next start.
- Cleaned compiler warnings; `cargo build` and `cargo test` are clean and green.

### Added — spec-007-free-and-open-for-all

- The person clarified the app is non-profit, open, and free for all. Added the
  `edition` module: one free edition, MIT license, and a full feature list
  where **every** feature resolves to `Free` — including capabilities normally
  sold as paid add-ons. No paid tiers, upgrades, billing, or accounts.
- Red: four tests in `src/edition.rs` were written first and observed failing;
  green: the policy constants and `tier_for` were corrected.
- UI: a "Free & open for all" card in the sidebar (click opens Settings) and a
  "Plans & access" section in Settings listing every feature with its `Free`
  tier. Added `LICENSE` and `license = "MIT"` to `Cargo.toml`.
- With 29 dark-theme reference pictures now available, the window defaults to
  the dark theme (light is still a toggle); `docs/unknowns.md` U-01 resolved.

### Changed — spec-008-modular-architecture

- The person noticed tests and implementation shared a file. **All eight
  `#[cfg(test)] mod tests` blocks were removed from `src/`** and relocated to
  one integration-test crate per module in `tests/` (`model`, `store`, `server`,
  `client`, `cli`, `edition`, `screenshot`, `ui`, `integration`), following the
  Rust Book ch. 11 test organization.
- DRY: four duplicated temporary-file helpers and a duplicated
  `load_or_seed_creates_file` test were replaced by single definitions in
  `tests/common/mod.rs` and `tests/store.rs`; envelope decoding now uses a
  shared helper.
- SRP: the 1700-line `ui.rs` was split into `ui/palette.rs`, `ui/icons.rs`,
  `ui/app.rs`, `ui/render.rs`, and `ui/dialogs.rs`; `ui.rs` is now a 16-line
  module root with declarations and re-exports.
- DIP / OCP: added the `WorkspaceStore` persistence port with `JsonStore` and
  `NoStore` adapters. `WorkspaceServer` now depends on the trait and exposes
  `with_store`; `new` and `with_path` keep their original behavior.
- Red: the four new port tests failed to compile (missing `WorkspaceStore`);
  green: the port, adapters, and server change were implemented.
- Researched Rust Book ch. 11/12 and Rust API Guidelines; recorded the
  architectural patterns in `AGENTS.md` and the evaluation of the suggested
  library list in `docs/adr-001-stack-evaluation.md` (agree: Rust, Serde;
  defer: Axum, PostgreSQL, SQLx, WebSocket, Yrs behind the trait seams;
  decline: Slint, Tauri, OAuth2 for this scope). No new dependencies.
- `cargo build` and `cargo test` are clean and green; a fresh `--screenshot`
  was compared with the 29 dark-theme reference pictures.

### Added — spec-009-visual-fidelity

- Re-opened all 29 reference pictures and rebuilt the window to match them:
  the window is now **1322×861** (the size of the pictures) instead of
  1280×800.
- Added a custom top tab strip (sidebar toggle, quick-note button with a
  notification dot, back/forward history, an active page tab, a new-tab
  button, and minimize / maximize / close controls).
- Reshaped the sidebar: a "Search or ask" box with a `Ctrl+K` hint, Library /
  Settings / Inbox rows, lowercase Favorites and Private sections, a
  "Customize sidebar" row, the free-edition card, and a local account row.
- Reshaped the breadcrumb bar (page icons, crumbs, Share with icon, link,
  star, more-actions) and removed the timestamp that the pictures do not
  show.
- Reshaped the page column: an original open-book icon drawn from shapes, a
  very large bold title, a callout with a small page icon, bold section
  headings, and block handles that appear only on hover. Neutral English demo
  content shaped like the pictured document page.
- Added the opened states the pictures show: a full-width **Library** table,
  a **Share** popover, an **Inbox** panel, and a wide two-column **Settings**
  window with a navigation rail. Back/forward page history and tab-title
  shortening were added too.
- Typography: Segoe UI is used when the operating-system fonts are present,
  with a bold "Strong" family for titles and headings; egui's built-in fonts
  remain the fallback.
- Added `ION_OF_NOTES_SHOW` (a harmless preview hook) so each opened state can
  be rendered with `--screenshot` and compared with the pictures. Normal runs
  and the rebuild check never set it.
- Red: six tests in `tests/fidelity.rs` were written first and observed
  failing (missing `chrome`, `strong_font`, `tab_title`, and the reshaped
  demo page). Green: `palette.rs`, `icons.rs`, `app.rs`, `render.rs`,
  `dialogs.rs`, `model.rs` and `main.rs` were implemented.
- Updated `docs/requirements.md` (FR-37..FR-42, NFR-03),
  `docs/unknowns.md`, `AGENTS.md` and `README.md`.

### Changed — spec-010-architecture-guidelines

- The person added an architecture review (Rust API Guidelines, Rust Book
  ch. 7, and a ChatGPT review of Modular Monolith / Clean Architecture /
  pragmatic DDD) and asked that improved project guidelines always live in
  `AGENTS.md`. Added them there: the two distinct decisions (code
  modularization vs business architecture), layer responsibilities and
  dependency direction, the `app`/bootstrap layer, Rust Book ch. 7 visibility
  rules (private by default, `pub(crate)`, `pub use` facade), file-creation
  heuristics, pragmatic DDD invariants and context errors, desktop
  presentation as an input adapter, the feature-based growth path, and a
  team-conventions table. Added both new references to External References.
- Applied the review with one contained refactor: new `src/app.rs` composition
  root (`build_server`/`build_desktop` and testable `*_at` variants) wires the
  JSON adapter to the server and client ports. `main.rs` (CLI wiring only) and
  `ui/app.rs` now delegate to it instead of duplicating the wiring (DRY / SRP).
- Moved `data_path()` out of `ui/palette.rs` (theming) into `store.rs` (the
  persistence adapter), since the data file location is an infrastructure
  decision; removed the stale `ui.rs` re-export and the unused import.
- Red: two tests in `tests/app.rs` were written first and observed failing
  (unresolved `ion_of_notes::app`); green: the composition root and its wiring
  were implemented. `cargo build` and `cargo test` are clean and green.
- Recorded the decision in `docs/adr-002-architecture-guidelines.md` and this
  session in `spec/spec-010-architecture-guidelines.md`.
- Pure refactor: no behavior, API, or visual change; a fresh `--screenshot`
  was compared with all 29 reference pictures.

### Changed — spec-011-collaborative-server (self-hosted client/server)

- The person de-emphasized "free & open for all": the app is **open source**
  (MIT) as a license fact, not a marketing feature, and the product is now a
  **self-hosted client/server with accounts**. Retired `spec-007` and removed
  the `edition` module, the sidebar "Free & open for all" card and the Settings
  "Plans & access" feature-tier list. The sidebar card is now "Invite members"
  and Settings has a neutral "Workspace & sync" section; the demo callout
  describes the self-hosted, syncing, collaborative server.
- **Decided the API** in `docs/adr-003-self-hosted-collaborative-api.md`:
  REST/JSON (`/api/v1`, accounts/sessions, envelopes) plus a WebSocket CRDT
  real-time channel, a versioned sync log, sharing/permissions DTOs, and the
  Notion-style export tree. `docs/adr-001-*` is annotated as superseded by this
  decision; `docs/unknowns.md` U-03/U-07 resolved.
- Red: ten tests in `tests/collab.rs` were written first and observed failing;
  green: a new `src/collab.rs` domain module (members/roles, shares/access,
  sync `Change`/`SyncBatch`, `RealtimeContract`, `ExportFile`, `CollabState`),
  its re-exports, and `page_markdown` / `people_csv` / `export_tree`.
- `WorkspaceServer` now owns a `CollabState`; all mutations go through one
  `write(op, page_id)` path that appends a `Change` (DRY / SRP). New routes
  `/api/members`, `/api/shares`, `/api/sync`, `/api/export`, `/api/realtime`
  and typed `Client` methods expose the decided contract. ADR-003 scopes the
  production Axum/PostgreSQL/WebSocket/Yrs adapters as later work behind the
  existing seams — no new dependency added.
- Export reproduces the supplied structure: one markdown file per page,
  "folder" pages linking to their children (URL-encoded paths), and a `People`
  folder with `People.md`, `People_all.csv` and one membership file per person.
- Updated `AGENTS.md` (overview, module tree, API contract table, references),
  `README.md`, `LICENSE`, `docs/requirements.md` (FR-34..FR-36, FR-43,
  NFR-04, out-of-scope notes), `docs/data-model.md`, `docs/unknowns.md`.
  Emphasized DRY / Clean Code / SOLID / modularization.
- `cargo build` clean, `cargo test` green; a fresh `--screenshot` was compared
  with all 29 reference pictures.

### Added — spec-012-interaction-polish

- The person asked to drop mail/calendar, integrate the AI assistant with
  Ollama, remove the duplicated native window frame, fix the misplaced `×`,
  implement the app-menu + help-site keyboard shortcuts, and give the tab-strip
  `+` the behaviour shown in the two attachments. Scope, evidence and
  Given/When/Then criteria are in `spec/spec-012-interaction-polish.md`.
- **Frameless window:** `main.rs` now builds the viewport with
  `.with_decorations(false)`; the Rust-drawn tab strip is the only frame and
  starts a window drag (`ViewportCommand::StartDrag`) when dragged.
- **Mail & Calendar** removed from the Settings rail; no mail/calendar feature
  is implemented (FR-45).
- **Ollama AI assistant:** new `src/ollama.rs` adapter detects either the
  `ollama` CLI in `PATH` or a service at a network `host:port` (`OLLAMA_HOST`,
  default `127.0.0.1:11434`). It builds `/api/generate` and `/api/tags`
  requests with the standard library only, parses plain and chunked HTTP
  replies, auto-selects an installed model, and runs on a background thread so
  the window stays responsive. Settings → AI assistant shows the connection
  status, endpoint, model and a test question; "Search or ask" can ask the
  model and display the answer. Verified end-to-end against the local Ollama
  service (a real answer was returned).
- **Keyboard shortcuts:** new pure `src/shortcuts.rs` table covers the app menu
  (File/Edit/View/History/Window/Help) and
  <https://www.notion.com/help/keyboard-shortcuts>. `ui/app.rs` translates
  exact key combinations into `Command`s and consumes the events so they do not
  leak into text editors. Added undo/redo, duplicate page, copy link/name,
  zoom, full screen, back/forward, reload, theme toggle and markdown prefixes
  (`# `, `## `, `* `, `[] `, …). `F1` opens an in-app shortcut sheet.
- **Add-tab `+`:** the tab strip now keeps a list of tabs. `+` / Ctrl+T opens
  the "Open in new tab…" switcher from the attachments (search field, filter
  row, New chat / New page, Today / Past week page groups, live preview pane,
  footer hints `Ctrl+↵ / Ctrl+L / Shift+Ctrl+K`). Rows open in a new tab;
  Ctrl+W closes a tab, Ctrl+Shift+T reopens it, Ctrl+Tab cycles tabs.
- **Preferences window:** the close `×` is now a fixed top-right control in a
  rounded well (matching `reference/picture-15..29`) instead of sitting on the
  heading row. The rail gained glyphs and is wider; Appearance / Input options
  / Language & time rows now mirror the reference layout.
- Red: `tests/shortcuts.rs` (6 tests) and `tests/ollama.rs` (7 tests) were
  written first; green: both modules implemented. `cargo build` clean,
  `cargo test` green, and a fresh `--screenshot` compared with all 29
  reference pictures. A stale demo `workspace.json` from an earlier session
  was reset so the default view re-seeds the clean 5-page workspace.

### Added — spec-013-interaction-polish-and-modularity

- **Architecture answer** (`docs/adr-004-modularization-decision.md` and
  `AGENTS.md`): the `tcg-matchmaker` worker tree is feature-first
  modularization (vertical slices), not DDD. The decision stays the pragmatic
  Modular Monolith of ADR-002: split a file when it holds more than one
  coherent responsibility, never by line count or one-struct-per-file; adopt
  feature-based `src/modules/<feature>/` only when a second bounded context
  appears. Applied with two contained extractions instead of a layer scaffold:
  `ui/switcher.rs` (one shared search / new-tab window — DRY) and
  `ui/page_icon.rs` (page-icon feature). `ui.rs` remains the small facade.
- **Bug — keyboard hijack:** `ui/app.rs` now only trusts a key as a shortcut
  when the window has keyboard focus, the pointer is over the window, and the
  event's modifiers agree with the frame's held modifiers (pure, tested
  `shortcuts::shortcut_context_ok`). Key events arriving after the pointer
  leaves the window, or with a stale `Ctrl`/`Alt` left over from a focus
  change, no longer turn ordinary typing into global commands.
- **Bug — sidebar overflow:** the sidebar page list is now a bounded vertical
  `ScrollArea`; the search box and the account / invite / customize group stay
  fixed when the list grows.
- **Compact top bar:** 36 px tab strip, no person/settings control (Settings
  lives in the sidebar), tighter breadcrumb row.
- **Shared search / new-tab window:** both opened states now use one renderer
  with a hint field, Today / Past week groups, a preview (highlight) pane, a
  footer of keyboard hints, and working **show/hide highlight pane** and
  **show/hide filters** toggles. `ION_OF_NOTES_SHOW=search|newtab` previews them.
- **Page actions:** the "..." control opens a searchable page-actions menu
  (Copy link, Duplicate, Move to Trash, Show/hide sidebar, …) with Appearance
  & layout (Default / Serif / Mono, Small text, Full width), Available offline
  / Lock page toggles and a Last edited footer. `ION_OF_NOTES_SHOW=pageactions`
  previews it.
- **Show / hide sidebar:** the tab-strip panel control (tooltip + `Ctrl+\\`)
  and the breadcrumb ">" control toggles it.
- **Block handles:** the "+" inserts a block **below** (Alt-click inserts
  **above**) and opens the block menu for the new block; the six-dot grip is
  now click-and-drag, reorders blocks to the hovered position, and opens a
  categorized Notion-style block menu (Suggested / Basic blocks / Media /
  Database / Advanced blocks / Inline / Embeds).
- **Customizable page icon:** clicking the large book icon opens an
  Emoji / Icons / Upload picker with a filter field, shuffle and Remove. The
  chosen icon is stored on the `Page` and rendered by one helper in the
  header, sidebar, breadcrumbs, tab strip and switcher preview. Segoe UI
  Symbol and Times New Roman were registered as fallback faces for the glyphs
  and the Serif font style. `ION_OF_NOTES_SHOW=iconpicker` previews it.
- Red: two tests in `tests/shortcuts.rs` were written first and observed
  failing; green: the pure guard and the UI wiring were implemented.
  `cargo build` clean, `cargo test` green (80 tests), and a fresh
  `--screenshot` was compared with all 29 reference pictures. A stale demo
  `workspace.json` was reset so the clean 5-page workspace re-seeds.

### Notes

- Visual design now follows the 29 reference pictures (dark theme, tab strip,
  sidebar, centered page column, and the Library / Share / Inbox / Settings
  opened states); see `docs/unknowns.md`.
- No code, assets, or text were taken from the original program (clean room).
- 78 tests pass (all in `tests/`, plus one end-to-end scenario); the red step
  for each spec was observed before implementation. `cargo build` and
  `cargo test` are clean and green, and `--screenshot` was compared by eye
  with the 29 reference pictures after each build.
