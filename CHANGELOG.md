# Changelog

All notable changes to this project will be documented in this file.
The format follows "Keep a Changelog" and the SDD cycle described in
`AGENTS.md`: spec → plan → tests (red) → code (green) → changelog → docs.

## [Unreleased]

### Changed — spec-024-library-views

- Library tabs are functional: Favorites filters favorite pages, Shared shows
  pages with a page-specific share, Private shows pages without one, and
  Recents shows the workspace order. The general `"*"` share does not mark
  every page as shared.
- Replaced the `More` tab with a menu offering AI Meeting Notes, Agents,
  Templates and Trash placeholder views.
- Library rows now show the owner from the member directory, the workspace
  name as Source, and the page's real `updated_at` as Last edited time.
- `OpenLibrary` refreshes shares/members; `NOT_NOTION_SHOW=library` populates
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
- Added `NOT_NOTION_SHOW=share` population for screenshot previews.
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
- Added `NOT_NOTION_SHOW` (a harmless preview hook) so each opened state can
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
  (unresolved `not_notion::app`); green: the composition root and its wiring
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
  **show/hide filters** toggles. `NOT_NOTION_SHOW=search|newtab` previews them.
- **Page actions:** the "..." control opens a searchable page-actions menu
  (Copy link, Duplicate, Move to Trash, Show/hide sidebar, …) with Appearance
  & layout (Default / Serif / Mono, Small text, Full width), Available offline
  / Lock page toggles and a Last edited footer. `NOT_NOTION_SHOW=pageactions`
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
  and the Serif font style. `NOT_NOTION_SHOW=iconpicker` previews it.
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
