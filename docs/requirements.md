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
- FR-02: A page has an id, title, icon glyph, optional parent, favorite flag,
  an ordered list of blocks, optional table rows, and an updated timestamp.
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
- NFR-05: The UI stays responsive; the workspace is small enough that a full
  snapshot per edit is cheap.
- NFR-06: All colors, fonts, and spacing come from a single palette module.
- NFR-07: The clean-room boundary is never crossed: no tool reads the original
  executable, its folder, its pages' code, or its assets.
- NFR-08: Implementation files contain no test code. Tests live in `tests/` as
  one integration-test crate per module; shared helpers are defined exactly once
  in `tests/common/mod.rs` (DRY).
- NFR-09: Core modules depend on trait ports, not concrete adapters
  (`WorkspaceApi`, `WorkspaceStore`), and each module has a single
  responsibility (DRY + SOLID).

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
