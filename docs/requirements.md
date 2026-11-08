# Requirements — not-notion

## 0. Scope and constraints

- R-00: This is a clean-room implementation. Nothing inside the original
  program, its files, or its folder is read, listed, run, or decompiled.
- R-01: The application is a desktop app. The system consists of a **client**
  (window + API client) and a **server** (workspace service + optional loopback
  HTTP listener).
- R-02: All documentation and all implementation files are written in English.
- R-03: Working product name is `not-notion`.
- R-04: No copied logos, brand names, photos, or long passages of text. Neutral
  shapes, gradients, and short sample copy only.
- R-05: Every feature is specified in `spec/` before implementation and built
  test-first (TDD). See `AGENTS.md`.

## 1. Functional requirements

### 1.1 Workspace and pages

- FR-01: The workspace has a name and holds an ordered set of pages.
- FR-02: A page has an id, title, icon glyph, optional parent, favorite,
  locked, offline, table-of-contents, discussions, inline-comments and archived
  flags, a list of page comments, an ordered list of blocks, optional table
  rows, and an updated timestamp.
- FR-03: Pages form a tree through `parent`; a page may have child pages.
- FR-04: The user can create a new page under any parent (or at the root).
- FR-05: The user can rename a page and change its icon.
- FR-06: The user can delete a page; deleting a page also deletes all of its
  descendants.
- FR-07: The user can mark a page as favorite / unfavorite it.
- FR-08: The sidebar shows favorites and the private page tree, and the tree
  can be expanded and collapsed.
- FR-09: Clicking a page in the sidebar opens it in the content area.
- FR-10: A breadcrumb trail shows the path from the root to the current page.

### 1.2 Blocks and editing

- FR-11: A page contains blocks of types: paragraph, heading 1/2/3, bulleted
  list, numbered list, to-do, quote, callout, code, and divider.
- FR-12: The user can edit block text directly in place.
- FR-13: The user can add a block after any block and append a block at the end.
- FR-14: The user can delete a block.
- FR-15: The user can change a block's type using a slash-command menu.
- FR-16: A to-do block has a checkbox; checking it toggles completion.
- FR-17: A code block renders in a monospace, shaded panel; a quote has a left
  bar; a callout has a shaded rounded panel; a divider is a horizontal rule.
- FR-18: The page title is editable in place and is displayed large.

### 1.3 Table database

- FR-19: A page can hold inline table rows (name, status, assignee, due date).
- FR-20: The user can view rows as a table and as a status board.
- FR-21: The user can add a new row and edit row cells.
- FR-22: Status values are shown as colored chips; clicking a chip cycles it
  through `Not started`, `In progress`, and `Done`.

### 1.4 Search and navigation

- FR-23: The user can open a quick-search dialog from the sidebar or Ctrl+K.
- FR-24: Search matches page titles and block text, case-insensitively, and
  ranks title matches above body matches.
- FR-25: Selecting a search result opens that page.
- FR-26: The user can switch between light and dark themes.

### 1.5 Client, server, and persistence

- FR-27: All UI data access goes through a `Client` that uses the `WorkspaceApi`
  contract; the UI never mutates the server's state directly.
- FR-28: The server exposes the routes listed in `AGENTS.md` and returns
  standard status codes with JSON envelopes.
- FR-29: The `--serve` flag starts a loopback HTTP/1.1 listener that dispatches
  the same requests to the same server.
- FR-30: The workspace is persisted as JSON and restored on the next start.
- FR-31: On first start (no saved file), a small neutral demo workspace is
  created.

### 1.6 Screenshot mode

- FR-32: `--screenshot <file.png>` starts the app normally, draws about 120
  frames, saves the window's picture as a PNG, and exits.
- FR-33: A build without `--screenshot` support, or a blank PNG, is a failure.

### 1.7 Self-hosted client/server collaboration (spec-011 / ADR-003)

- FR-34: The application is open source (MIT). Being open source is a license
  fact, **not** emphasized in the UI: there is no free-edition card and no
  plan/feature-tier list. The window defaults to the dark theme in `reference/`.
- FR-35: The system is a client/server architecture with a **self-hosted
  server** and **accounts**. It provides a People directory (`Member`, `Role`),
  sharing with permissions (`Share`, `Access`), a versioned document
  synchronization log (`Change`, `SyncBatch`), and real-time collaboration.
- FR-36: The API is **decided** in `docs/adr-003-*`: REST/JSON resources under
  `/api/v1` (loopback unversioned equivalents today) plus a WebSocket CRDT
  channel. The UI shows an "Invite members" card and a "Workspace & sync"
  Settings section, matching the pictured accounts/sharing surfaces.
- FR-43: `GET /api/export` returns the Notion-style export tree: one markdown
  file per page, folder pages linking to their children, and a `People` folder
  with membership files and a `_all.csv`.

### 1.8 Window chrome and opened states (spec-009)

- FR-37: A custom top strip spans the whole window: sidebar toggle, quick-note
  button, back/forward navigation, one active page tab, a new-tab button, and
  minimize / maximize / close controls.
- FR-38: The sidebar starts with a "Search or ask" box showing a `Ctrl+K` hint
  and ends with a "Customize sidebar" row, the invite-members card, and a local
  account row. Favorites and the private page tree are shown between them.
- FR-39: The breadcrumb bar shows the page path on the left and Share, link,
  star and more-actions controls on the right. The page column is centered,
  with a large book icon above a very large bold title. Block handles appear
  only while a block row is hovered.
- FR-40: The user can open the Library table, the Share popover, the Inbox
  panel, and a two-column Settings window; each can be closed again.
- FR-41: Back and forward navigate a history of visited pages; long page
  titles are shortened with an ellipsis in the tab strip.
- FR-42: Titles and section headings use a strong display face when one is
  available from the operating system; otherwise egui's built-in fonts are
  used.

### 1.9 Frameless window, tabs, shortcuts and the AI assistant (spec-012)

- FR-44: The OS window decorations are disabled (`with_decorations(false)`).
  The whole custom tab strip is the frame; it starts a window drag when dragged.
  No second, native frame is drawn.
- FR-45: Mail & Calendar is **out of scope**: the Settings rail offers no
  "Mail & Calendar" entry and no mail/calendar feature is implemented.
- FR-46: The tab-strip `+` (and Ctrl+T) opens an "Open in new tab…" switcher
  with a search field, filter row, "New chat" and "New page" actions, a recent
  page list grouped by Today / Past week, a live preview pane and a footer of
  keyboard hints (`Ctrl+↵`, `Ctrl+L`, `Shift+Ctrl+K`).
- FR-47: Activating a switcher row adds a tab showing the chosen page (or a new
  page / the AI chat). Tabs are selectable, closable (Ctrl+W) and reopenable
  (Ctrl+Shift+T); Ctrl+Tab / Ctrl+Shift+Tab cycle them.
- FR-48: Keyboard shortcuts from the app menu (File / Edit / View / History /
  Window / Help, `reference/picture-6..11`) and from
  <https://www.notion.com/help/keyboard-shortcuts> are implemented through one
  pure table (`shortcuts.rs`) and translated into `Command`s. `F1` opens an
  in-app shortcut sheet; markdown prefixes (`# `, `* `, `[] `, …) change block
  types.
- FR-49: The AI assistant is integrated with Ollama: either the `ollama` CLI
  in `PATH` or a service at a network `host:port` (`OLLAMA_HOST`). The adapter
  (`ollama.rs`) detects the transport, lists models and asks `/api/generate`.
  The configured model/endpoint and the connection status are shown in the
  Settings → AI assistant page; requests run on a background thread.
- FR-50: The preferences window follows `reference/picture-15..29`: a wider
  navigation rail with glyphs, and the close `×` is a fixed control in the
  top-right corner (in a rounded well), never on the heading row.
- FR-51: Undo / Redo (Ctrl+Z / Ctrl+Shift+Z), duplicate page (Ctrl+D),
  copy link/name (Ctrl+L / Alt+Ctrl+L), zoom (Ctrl++/-/0), full screen (F11),
  theme (Ctrl+Shift+L), back/forward (Ctrl+[ / Ctrl+]) and reload (Ctrl+R) are
  reachable from the keyboard.

### 1.10 Interaction polish, bugs and modularity (spec-013)

- FR-52: Keyboard shortcuts are only trusted while the window has keyboard
  focus, the pointer is over the window, and the event's modifiers agree with
  the frame's held modifiers; plain keystrokes are never stolen from a focused
  text editor (pure, tested guard in `shortcuts.rs`).
- FR-53: The sidebar page list scrolls when it overflows; the search box and
  the bottom account / invite / customize group stay fixed.
- FR-54: The top bar is compact (36 px) and has no person/settings control;
  Settings is reached from the sidebar.
- FR-55: The search window and the "+" new-tab window share one layout
  (hint field, Today / Past week groups, preview pane, footer hints), with
  working show/hide highlight-pane and show/hide filters toggles (DRY,
  `ui/switcher.rs`).
- FR-56: The sidebar can be shown/hidden from the tab strip (and Ctrl+\).
- FR-57: The "..." control opens a searchable page-actions menu with
  Copy link, Copy page contents, Duplicate, Move to Trash, Appearance & layout
  (Default / Serif / Mono, Small text, Full width), Available offline /
  Lock page toggles, Archive, Version history and a Last edited footer.
- FR-58: The block "+" inserts a block below (Alt-click: above) and opens the
  block menu; the six-dot grip reorders blocks by drag and opens a
  categorized block menu on click.
- FR-59: The page icon is customizable (Emoji / Icons / Upload picker with a
  filter and Remove) and the choice is replicated into the sidebar,
  breadcrumbs, tab strip and switcher preview.
- FR-60: Code is divided by coherent responsibility, not by line count or DDD
  scaffolding (ADR-004): `ui/switcher.rs` and `ui/page_icon.rs` are separate
  cohesive modules behind the `ui.rs` facade.
- FR-61: The Settings search field filters the navigation rail by section title
  or description, moves to the first match, and reports when nothing matches
  (`ui/settings_search.rs`, spec-015).
- FR-62: The page actions menu opens a version-history window backed by
  `GET /api/pages/{id}/versions`; snapshots are listed newest first and the
  selected snapshot can be restored through `Client::update_page` (spec-015).

### 1.11 Architecture follow-up and clean-room observation (spec-016)

- FR-63: Collaboration state (members, shares, sync log, page versions) is
  persisted through a `CollabStore` port and restored on restart; a failing
  collaboration store returns an API `500` (implemented in `spec-019`).
- FR-64: A single-page edit is sent with `PATCH /api/pages/{id}` and persisted
  with debounce; full-workspace replacement is reserved for added, removed or
  reordered pages (implemented in `spec-020`).
- FR-65: Page lookup, children and breadcrumbs use a derived workspace index
  without changing the JSON shape (implemented in `spec-022`).
- FR-66: Version history has a documented retention limit (50 per page) and a
  restore is recorded as a new version (implemented in `spec-022`).
- FR-67: `ui/app.rs` is split into cohesive `commands`, `view`, `input` and
  `apply` modules behind the `ui.rs` facade; `ui/dialogs.rs` and
  `ui/render.rs` are facades over cohesive submodules (implemented in
  `spec-017`, planned in spec-016 A5).
- FR-68: Core user-visible strings (sidebar, Share, Library, page actions,
  comments and Settings) are covered by the locale key table with an English
  fallback, checked by a completeness test; remaining long-form copy is
  migrated incrementally (implemented in `spec-028`, originally planned in
  spec-016 A6).
- FR-69: Client serialization failures return an explicit `ClientError::Encode`
  instead of sending an empty body (implemented in `spec-021`).
- FR-70: The optional loopback listener serves concurrent connections without
  blocking one client behind another (implemented in `spec-021`).
- FR-71: Settings search filters the rows inside the open section, not only the
  navigation rail (**open**: spec-016 A9; the rail filter is implemented, the
  row filter is not).

### 1.12 Sharing dialog (spec-023)

- FR-72: The Share popover loads the member directory and general access,
  supports comma-separated emails with Guest/Member roles, sends invites
  through the client, refreshes the list, shows status/errors, cycles general
  access through Full/Can edit/Can view/No access, and copies the page link
  (implemented in `spec-023`). The optional invite message is UI-only until
  `MemberDraft` carries a note field.

### 1.13 Library views (spec-024)

- FR-73: The Library tabs are functional (Recents / Favorites / Shared /
  Private), the More menu offers placeholder views, and rows show the owner,
  workspace source and real last-edited time (implemented in `spec-024`).

### 1.14 Settings pages (spec-025)

- FR-74: Workspace settings (name, icon, landing page, allowed domains,
  sidebar visibility, analytics, offline downloads and People toggles) are
  editable and persisted with the workspace; General / Sidebar / People /
  Offline pages are functional (implemented in `spec-025`). Workspace deletion
  remains disabled in this build.

### 1.15 Page actions and comments (spec-026)

- FR-75: Page actions support Copy page contents, a rendered Table of
  contents, Page discussions with add/delete comments, Inline comments toggle,
  Archive/restore and status feedback for placeholder actions; archived pages
  are hidden from normal views and listed in the Library Archive view
  (implemented in `spec-026`). Block-anchored inline comments, notifications,
  analytics and move-to remain follow-ups.

### 1.16 Internationalization coverage (spec-028)

- FR-76: Core UI labels and status messages for the sidebar, Share, Library,
  page actions, comments and Settings use a shared key table with English,
  Portuguese (Brazil) and Esperanto translations; key completeness and locale
  fallback are tested (implemented in `spec-028`). Long-form demo copy and
  remaining strings continue to be migrated incrementally.

### 1.17 Onboarding and templates (spec-027)

- FR-77: An empty, not-yet-onboarded workspace shows “What is this space for?”
  with For work / For personal life / For school; the choice persists and
  applies a starter template. A template library offers work, personal,
  course, meeting and blank templates that create real pages (implemented in
  `spec-027`). Custom templates and template search remain follow-ups.

### 1.18 Public API alignment (specs 030–037; 030 implemented)

These requirements come from the B4 public-documentation review
(`docs/public-api-analysis.md`, ADR-006). Spec-030 (versioning, errors,
capabilities) is implemented; the rest are planned.

- FR-78: `/api/v2` requires a `Not-Notion-Version` header and returns a
  structured error object with `status`, `code`, `message`, `request_id` and
  optional `additional_data` (implemented in `spec-030`).
- FR-79: Connections have capabilities for content read/update/insert, comment
  read/insert and user-info visibility; permission failures return `401`/`403`
  with stable codes (foundation implemented in `spec-030`; comment/user-info
  routes arrive in spec-035).
- FR-80: List endpoints are cursor-paginated with `start_cursor`, `page_size`
  (1–100), `results`, `has_more` and an opaque `next_cursor` (implemented in
  `spec-031`).
- FR-81: Blocks use rich text objects (`text`, `mention`, `equation`) with
  annotations, `plain_text` and `href` (implemented in `spec-032`).
- FR-82: Blocks carry `object`, `id`, `parent`, timestamps, users, `in_trash`,
  `has_children` and a typed payload; block children are read/appended through
  dedicated endpoints (implemented in `spec-032`).
- FR-83: Pages in a data source carry typed property values; property items
  paginate large relation/people/rich-text values (spec-033).
- FR-84: A database is a container of data sources; each data source owns its
  property schema and rows (spec-033).
- FR-85: Views define table/board/calendar/list layouts with filters, sorts and
  configuration (spec-034).
- FR-86: Comments attach to pages or blocks, carry rich text, a discussion id,
  up to three attachments and an optional display name (spec-035).
- FR-87: Users distinguish people and bots, expose optional profile fields and
  respect user-info capabilities (spec-035).
- FR-88: Files are `file`, `file_upload` or `external`; uploads have a
  pending/uploaded/expired/failed lifecycle and temporary URLs (spec-035).
- FR-89: Webhook subscriptions verify an endpoint, sign payloads with
  HMAC-SHA256 and retry failed deliveries (spec-036).
- FR-90: The server enforces rate, request-size and workspace block limits and
  returns `429`/`529`/`503` with retry guidance (spec-037).
- FR-91: v1 routes stay stable; v2 changes are additive unless a version bump
  is declared; clients ignore unknown response fields and treat cursors as
  opaque (versioning and v1 stability in `spec-030`; cursor opacity in
  `spec-031`).
- FR-92: Unsupported block or view types are represented explicitly instead of
  being silently dropped (block side implemented in `spec-032`; view side in
  spec-034).
- FR-93: Page and callout icons use an icon object (`emoji`, `custom_emoji`,
  `icon`, `external`, `file`) rather than a bare glyph string (planned in
  spec-035).
- FR-94: Parent objects distinguish page, block, database, data source,
  workspace and agent parents (page/block implemented in `spec-032`; the rest
  planned in specs 033/035).

## 2. Non-functional requirements

- NFR-01: `cargo build` succeeds on stable Rust with no manual steps.
- NFR-02: `cargo test` passes and covers model, store, server, client, and PNG
  output.
- NFR-03: The window opens at 1322x861 logical points (the size of the
  reference pictures) and remains usable down to 900x600.
- NFR-04: The domain has no dependency on a particular transport. v1 runs
  offline with an in-process / loopback server; the self-hosted production
  server, accounts and network/auth concerns are adapters behind `WorkspaceApi`
  (ADR-003), never domain rules.
- NFR-05: The UI stays responsive: a single-page edit is sent with `PATCH` and
  a 250 ms debounce, and the full snapshot is reserved for structural or
  settings changes (`spec-020`).
- NFR-06: All colors, fonts, and spacing come from a single palette module.
- NFR-07: The clean-room boundary is never crossed: no tool reads the original
  executable, its folder, its pages' code, or its assets.
- NFR-08: Implementation files contain no test code. Tests live in `tests/` as
  one integration-test crate per module; shared helpers are defined exactly once
  in `tests/common/mod.rs` (DRY).
- NFR-09: Core modules depend on trait ports, not concrete adapters
  (`WorkspaceApi`, `WorkspaceStore`, `CollabStore`), and each module has a
  single responsibility (DRY + SOLID).
- NFR-10: Every completed spec is committed to git in the same session
  (`AGENTS.md` → Commits).
- NFR-11: Search results are cached by query and workspace version; an
  unchanged frame must not call the server again (`ui/search_cache.rs`).
- NFR-12: JSON saves are atomic (temporary file + move) and a storage failure
  is reported as an API `500`, never as a silent success.
- NFR-13: `cargo fmt --check` is clean; formatting-only changes are committed
  separately from behavior changes (spec-016 A10).
- NFR-14: Server behavior is learned through public documentation, public
  behavior, exports the person already owns, and our own decided contract.
  Own-account HAR capture is **not authorized** under the strict B1 scope; if
  the scope changes, redaction and the ADR-005 checklist apply. Interception
  tools are used only on traffic we control; Wireshark is not the primary
  discovery tool and protection mechanisms are never bypassed (ADR-005,
  spec-016 Track B).
- NFR-15: An unchanged frame does not call the server, a single-page edit does
  not send a whole-workspace payload, and the loopback listener does not
  serialize independent clients (spec-016 A2/A3/A8).
- NFR-16: Documentation stays current and navigable: `docs/index.md` is the
  entry point, each concern has one owner document, and a behavior/route/
  entity/module change updates that document in the same spec (spec-029).
- NFR-17: Public API alignment is evidence-based: only public documentation,
  UI observation or our own decision is used; no private endpoint, capture or
  decompiled behavior is claimed as fact (ADR-006, `docs/public-api-analysis.md`).
- NFR-18: v2 limits are explicit and testable: request size, rate, block and
  schema limits return stable codes and retry guidance (spec-037).
- NFR-19: v2 is additive and backward-compatible with v1; clients ignore
  unknown fields and treat cursors as opaque (spec-030/031).
- NFR-20: Legacy data (`Block.text`, `Page.rows`) migrates to the richer v2
  model without loss (spec-032/033).

## 3. Requirement → spec map

| Requirement                    | Spec                                  |
| ------------------------------ | ------------------------------------- |
| R-00..R-05, NFR-01, NFR-02    | `spec-001-project-foundation.md`      |
| FR-01..FR-10                  | `spec-002-domain-model.md`            |
| FR-27..FR-31                  | `spec-003-workspace-server.md`        |
| FR-27, FR-30                  | `spec-004-client-layer.md`            |
| FR-11..FR-26, NFR-03, NFR-06  | `spec-005-desktop-ui.md`              |
| FR-32, FR-33                  | `spec-006-screenshot-mode.md`         |
| FR-34..FR-36, FR-43           | `spec-011-collaborative-server.md`    |
| R-05, NFR-02, NFR-08, NFR-09  | `spec-008-modular-architecture.md`    |
| FR-37..FR-42, NFR-03, NFR-06  | `spec-009-visual-fidelity.md`         |
| FR-44..FR-51                  | `spec-012-interaction-polish.md`      |
| FR-52..FR-60, NFR-10          | `spec-013-interaction-polish-and-modularity.md` |
| FR-61, FR-62, NFR-11, NFR-12  | `spec-015-hardening-and-performance.md` |
| FR-63..FR-71, NFR-13..NFR-15  | `spec-016-architecture-follow-up-and-observation.md` |
| FR-68 (implemented)           | `spec-028-i18n-coverage.md` |
| FR-71 (open)                  | `spec-016-architecture-follow-up-and-observation.md` A9 |
| NFR-16 (documentation)        | `spec-029-documentation-and-contributor-guide.md` |
| FR-78, FR-79, FR-91, FR-94, NFR-17, NFR-19 | `spec-030-api-v2-foundation.md` |
| FR-80, NFR-19                 | `spec-031-pagination.md` |
| FR-81, FR-82, FR-92, FR-93, FR-94, NFR-20 | `spec-032-rich-text-and-blocks.md` |
| FR-83, FR-84, NFR-20          | `spec-033-page-properties-and-data-sources.md` |
| FR-85, FR-92                  | `spec-034-views-and-database-layouts.md` |
| FR-86..FR-88, FR-93           | `spec-035-comments-users-and-files.md` |
| FR-89                         | `spec-036-webhooks-and-event-delivery.md` |
| FR-90, NFR-18                 | `spec-037-limits-and-operations.md` |
| FR-67 (implemented)           | `spec-017-ui-modularization.md` |
| FR-63 (implemented)           | `spec-019-collaboration-persistence.md` |
| FR-64 (implemented)           | `spec-020-per-page-writes-and-debounce.md` |
| FR-69, FR-70 (implemented)    | `spec-021-quality-and-concurrency-baseline.md` |
| FR-65, FR-66 (implemented)    | `spec-022-workspace-index-and-version-retention.md` |
| FR-72 (implemented)           | `spec-023-add-members-sharing-dialog.md` |
| FR-73 (implemented)           | `spec-024-library-views.md` |
| FR-74 (implemented)           | `spec-025-settings-pages.md` |
| FR-75 (implemented)           | `spec-026-page-actions-and-comments.md` |
| FR-76 (implemented)           | `spec-028-i18n-coverage.md` |
| FR-77 (implemented)           | `spec-027-onboarding-and-templates.md` |

## 4. Scope notes (see `docs/unknowns.md` and ADR-003)

- **Decided direction (ADR-003):** accounts, a self-hosted server, document
  sync, sharing/permissions and real-time co-editing. The API contract, DTOs
  and a tested in-process slice are implemented in `spec-011`. The production
  Axum / PostgreSQL / WebSocket / Yrs adapters and auth middleware are staged
  behind the existing seams, not built in this version.
- Rich media uploads and embedded third-party content.
- Mobile or web clients.
- Pricing, billing, upgrade, trial, or paywall surfaces remain out of scope
  (the app is open source; the retired `spec-007` "free edition" marketing is
  no longer reproduced).
- `spec-007` is **retired**: its "non-profit, free for all, no accounts"
  framing is superseded by the self-hosted accounts decision in ADR-003.
- **Public API alignment (planned):** FR-78..FR-94 and NFR-17..NFR-20 come from
  the B4 public-documentation review and ADR-006. They are scheduled in
  specs 030–037 and are not part of the current v1 behavior.
