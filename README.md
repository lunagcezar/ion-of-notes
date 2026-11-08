# not-notion

A clean-room desktop workspace app: pages made of editable blocks, a page-tree
sidebar, quick search, and an inline table database. Built in Rust with
eframe/egui. The system has a client (window + API client) and a server
(in-process workspace service + optional loopback HTTP listener).

Everything is written in English and built spec-first (`spec/`) and test-first
(`cargo test`). See `AGENTS.md` for the full workflow.

## Architecture

The code follows ports-and-adapters with DRY and SOLID applied throughout:

- `src/` holds implementation only. `ui/` splits the window into `palette`,
  `icons`, `commands`, `view`, `input`, `app` (state shell), `apply` (command
  application), `render/` (chrome, sidebar, content, Library, database),
  `dialogs/` (settings, sharing, page actions, menus, help, search window),
  `widgets`, `switcher`, `page_icon`, `search_cache` and `settings_search` —
  each with one responsibility.
- `model.rs` keeps the entities and tree behavior; `model/seed.rs` holds the
  demo workspace and `model/search.rs` the ranked search. `collab.rs` keeps
  the collaboration DTOs/state and `collab/export.rs` the export tree.
  `server.rs` keeps the API routes; `server/http.rs` is the loopback HTTP
  adapter and `server/url.rs` the shared URL encoding.
- `WorkspaceApi` is the transport port; `WorkspaceStore` and `CollabStore` are
  the persistence ports. `JsonStore`/`JsonCollabStore` and
  `NoStore`/`NoCollabStore` are adapters; different backends can be injected
  with `WorkspaceServer::with_stores` without editing the core.
- `workspace.json` and `collab.json` are separate documents. A corrupt
  collaboration file cannot destroy pages, and a failed collaboration write
  is reported as an API `500`.
- Single-page edits persist with `PATCH /api/pages/{id}` and a 250 ms debounce;
  the full-workspace `PUT` is reserved for added, removed or reordered pages.
- `--serve` reads each request on its connection thread and locks the server
  state only for `handle`, so independent clients do not block each other's
  I/O. Serialization failures surface as `ClientError::Encode`.
- Page lookups, children and breadcrumbs use a private derived workspace index;
  version history keeps the newest 50 snapshots per page.
- The Share popover is functional: it loads members, invites comma-separated
  emails with Guest/Member roles, cycles general access and copies the page
  link. The optional invite message is UI-only for now.
- Library tabs filter Recents / Favorites / Shared / Private, the More menu
  offers placeholder views, and rows show the owner, workspace source and real
  last-edited time.
- Tests live in `tests/`, one integration-test crate per module, with shared
  helpers in `tests/common/mod.rs`. No tests are mixed into source files.
- `docs/adr-001-stack-evaluation.md` records the decision on a suggested
  library list (Slint/Tauri/Axum/PostgreSQL/SQLx/WebSocket/Yrs/OAuth2).
- `docs/adr-003-self-hosted-collaborative-api.md` decides the self-hosted
  collaborative API (accounts, sync, sharing, real-time, export);
  `src/collab.rs` holds the domain and its DTOs.

## Self-hosted client/server and open source

`not-notion` is **open source** under the **MIT** license (see `LICENSE`).
Being open source is a license fact, not a marketing feature: the window does
not advertise a "free edition".

The product is a **client/server architecture** with a **self-hosted server**
that provides **accounts**, **document synchronization** across devices,
**sharing** with permissions, and **real-time collaborative editing**. The
decided API (REST/JSON plus a WebSocket CRDT channel, with an `/api/v1`
production shape) is recorded in
`docs/adr-003-self-hosted-collaborative-api.md`; the loopback `--serve` adapter
exposes the same contract today. Collaboration DTOs and the Notion-style
export tree live in `src/collab.rs`.

- The window follows the 29 dark-theme reference pictures in `reference/`;
  it can be toggled back to light in Settings.
- The sidebar shows an "Invite members" card and Settings a "Workspace & sync"
  section — matching the pictured accounts/sharing surfaces, without the
  retired plan/feature-tier list.

## Build and run

```sh
cargo build
cargo test
cargo run
```

CI (`.github/workflows/ci.yml`) runs formatting, clippy, tests and a release
build on Windows.

## Other modes

```sh
# Save a picture of the window after ~120 frames and exit:
cargo run -- --screenshot shots/now.png

# Serve the workspace API on loopback (Ctrl+C to stop):
cargo run -- --serve 127.0.0.1:8787
```

The workspace is saved as JSON next to the app data on disk. On first start a
small neutral demo workspace is created.

## Previewing the pictured states

The window follows the 29 dark-theme reference pictures: a top tab strip, a
sidebar, a breadcrumb bar, a centered page column, and the Library, Share,
Inbox and Settings opened states. To capture one of those states with
`--screenshot`, set `NOT_NOTION_SHOW` to `library`, `share`, `inbox`,
`settings`, `search` or `tasks`:

```sh
NOT_NOTION_SHOW=settings cargo run -- --screenshot shots/settings.png
```

Normal runs never set this variable, so the default window is unchanged.
