# API Observations — not-notion

## Purpose

This is the clean-room behavioral inventory for the server side. It records
what the product UI and public behavior imply, and what our own decided
contract (ADR-003) specifies. It deliberately contains **no captured traffic,
no proprietary payloads, and no original source or assets**.

This file is a living document. Every entry is marked with its evidence level
so an inference is never mistaken for a fact about the original service.

## Method and scope

- **B1 decision (2026-11-07): public behavior only — strict clean room.**
- **B4 review (2026-11-08): public Notion developer docs reviewed.** Public
  documentation is allowed; no capture or private source was used.
- **Allowed sources:** the 98 `reference/` pictures, the 72 local
  `screenshots/` pictures, behavior visible in the product UI, public
  help/documentation pages, exports the person already owns, and the project's
  own decided contract in ADR-003.
- **Not allowed:** HAR or packet capture, `mitmproxy`/Charles/Wireshark,
  browser source maps or bundled code, decompilation, or bypassing any
  authentication, certificate pinning, rate limit or terms.
- **No original server response has been observed.** The inventory is built
  from UI implications and the decided contract, not from captured traffic.

### Evidence levels

| Level | Meaning |
| ----- | ------- |
| **UI-observed** | Visible in the reference pictures or normal product behavior. |
| **Observed (public docs)** | Stated by a public Notion developer page. This is public contract evidence, not private behavior. |
| **Inferred** | A reasonable server implication of the UI, not confirmed by public documentation. |
| **Decided** | Our own contract in ADR-003 / ADR-006. This is a design decision, **not** a claim about the original. |
| **Unknown** | Needs public documentation or a future authorized source. |

## Evidence used in this first inventory

- `reference/picture-1.png` … `reference/picture-98.png`: sidebar, page,
  breadcrumb, Share, Inbox, quick search, Library, Settings, People,
  export/offline and the later-build surfaces.
- `screenshots/picture-1.png` … `screenshots/picture-72.png`: a later UI build
  with the new-tab switcher, Add members, page actions, Settings (General,
  Sidebar, People, Language & time, Offline), Library tabs, onboarding,
  template library, AI/agent surfaces and slash/icon pickers. Summarized in
  `docs/visual-reference-inventory.md`.
- `docs/requirements.md`: FR-01..FR-77, especially the workspace, search,
  sharing, sync and export requirements.
- `docs/adr-003-self-hosted-collaborative-api.md`: decided REST/JSON and
  WebSocket contract.
- `docs/data-model.md`: `Workspace`, `Page`, `Block`, `TableRow`, `Member`,
  `Share`, `Change`, `PageVersion`, `ExportFile`.
- `spec/spec-011-collaborative-server.md` and
  `spec/spec-015-hardening-and-performance.md`: implemented in-process slice.

## Inventory

| Operation | UI / public evidence | Inferred behavior | Status | Confidence | Open questions |
| --------- | -------------------- | ----------------- | ------ | ---------- | -------------- |
| Load workspace | The window opens with the sidebar tree, favorites and the current page. | A full workspace or page-tree snapshot is loaded at startup. | **Decided:** `GET /api/workspace` | High for our contract; unknown for the original | Does the original send a snapshot, a delta, or separate page-tree and page calls? |
| List pages / tree | Sidebar shows Favorites and a nested Private tree. | A page-summary list with `id`, `title`, `icon`, `parent`, `favorite`. | **Decided:** `GET /api/pages` | High for our contract | Are children embedded, paginated, or fetched lazily? |
| Read one page | Opening a page shows its blocks and inline database. | A page resource containing blocks and rows. | **Decided:** `GET /api/pages/{id}` | High for our contract | Does the original load blocks separately from page metadata? |
| Create page | "New page" in the sidebar and the tab switcher. | Create from a title/icon/parent draft, return the new page. | **Decided:** `POST /api/pages` with `PageDraft` | High for our contract | Does the original create a default empty block, and what is its id strategy? |
| Update page / title | Inline title editing. | Update one page resource. | **Decided:** `PATCH /api/pages/{id}` | High for our contract | Does the original send a full page, a JSON patch, or a block-level operation? |
| Update block text | Clicking a block edits it in place; changes save automatically. | Per-block or per-page update. | **Inferred:** page/block update | Medium | Is persistence debounced? Is there a block-level endpoint? |
| Delete page / Move to Trash | Page actions list "Move to Trash". | Delete the page and its descendants, or mark them trashed. | **Decided:** `DELETE /api/pages/{id}` (subtree removal) | Medium | Does the original use a trash flag, a separate trash resource, or hard delete? |
| Search | "Search or ask" box and `Ctrl+K`; results are ranked. | Case-insensitive title/body search with ranking. | **Decided:** `GET /api/search?q=` | High for our contract | Does the original search titles, blocks, databases, comments, or all of them? Is it server-side or local? |
| People / members | People section in Settings; Share popover lists people. | A member directory with roles. | **Decided:** `GET /api/members`, `POST /api/members` | High for our contract | What are the exact roles and invitation states? |
| Sharing / permissions | Share popover with full/edit/view/no-access choices. | Grants keyed by page and subject. | **Decided:** `GET /api/shares?page=`, `PUT /api/shares` | High for our contract | Does the original support workspace-wide, teamspace, page, block and database permissions? |
| Sync | The app describes a self-hosted server syncing changes across devices. | A versioned change log with a cursor. | **Decided:** `GET /api/sync?since=` | Medium | Is the original cursor numeric, timestamped, opaque, or per-document? |
| Export | Export tree described in `docs/data-model.md`; People folder with CSV. | Markdown pages, folder pages, People files. | **Decided:** `GET /api/export` | High for our export shape; unknown for the original | Which exact filenames, metadata and CSV columns does the original use? |
| Real-time collaboration | Requirements and Settings describe live co-editing. | A WebSocket channel with presence/edit/awareness/sync events. | **Decided:** `GET /api/realtime`; `WS /ws/workspace/{id}` | Medium | What is the actual frame format, auth handshake and conflict model? |
| Version history | Page actions menu includes "Version history"; snapshots are listed and restorable. | Per-page version snapshots. | **Decided/implemented:** `GET /api/pages/{id}/versions` | High for our contract | Does the original store diffs, snapshots, or CRDT history? |
| Auth / sessions | Accounts and a self-hosted server are the decided direction. | Sign-in/session before workspace access. | **Decided in ADR-003, not implemented** | Medium | What are the original login, token refresh and session shapes? |
| Health / liveness | No UI evidence; needed by our loopback server. | A simple health probe. | **Decided/internal:** `GET /health` | High for our server | Not expected to match the original. |
| Errors | The UI shows no raw server errors. | JSON envelope with `data` and `error`. | **Decided:** `{ "data": …, "error": null \| "message" }` | High for our contract | What are the original status codes, error codes and retry semantics? |
| Pagination / cursors | Large workspaces are not represented in the pictures. | Likely needed for pages, search, members and sync. | **Unknown** | Low | Does the original paginate? With what parameter names and cursor shape? |

## What the UI tells us directly

- A workspace has a name and a page tree; pages have titles, icons, parents,
  favorites, blocks and optional table rows.
- Pages can be created, opened, renamed, favorited, duplicated, trashed,
  locked and marked available offline.
- Search is reachable from the sidebar and `Ctrl+K`; results open pages.
- Sharing is reachable from the breadcrumb bar and the invite card.
- Settings exposes People, workspace, offline and AI-assistant surfaces.
- The export tree described in `docs/data-model.md` is part of the expected
  product behavior (markdown pages, folder pages, People CSV).

These are **UI-observed**. They do not prove the original server's routes,
payloads or status codes.

## What ADR-003 decides

ADR-003 is our contract, not a description of the original server:

- REST/JSON under `/api/v1` in production; loopback unversioned equivalents
  today.
- JSON envelopes with `data` and `error`.
- Accounts/sessions, members/roles, sharing/permissions.
- A versioned sync log with a cursor.
- A WebSocket CRDT channel for real-time editing.
- A Notion-style export tree.

## Public API documentation review (B4, 2026-11-08)

The public Notion developer pages were reviewed with `webfetch`. They are an
allowed clean-room source because they are published for developers. The full
analysis is `docs/public-api-analysis.md`; this is the evidence summary.

### Observed (public docs)

| Area | Public fact | Implication for us |
| ---- | ----------- | ------------------ |
| Conventions | HTTPS `api.notion.com`, Bearer token, required `Notion-Version`, `object` discriminator, snake_case, ISO 8601 | Add a version header and structured objects |
| Errors | `object: "error"` with `status`, `code`, `message`, `request_id`, `additional_data` | Replace the string-only error envelope |
| Pagination | `start_cursor`, `page_size` 1–100, `results`, `has_more`, `next_cursor`, opaque cursors | Paginate every list endpoint |
| Capabilities | read/update/insert content, read/insert comments, user-info levels | Replace coarse `Access` with capability checks |
| Blocks | metadata + type object + children; 30+ types; rich text | Add rich text, metadata, nesting, block endpoints |
| Page | properties, icon, cover, parent, in_trash, url, public_url | Add page properties and a parent object |
| Properties | 25+ types; write/clear rules; property-item pagination | Replace `TableRow` with a property schema |
| Database | container of one or more data sources | Add database/data-source split |
| Views | table/board/calendar/timeline/gallery/list/form/chart/map/dashboard; filter/sorts/config | Add a `View` DTO |
| Comments | page/block parent, discussion, rich text, up to 3 attachments, display name | Expand comments beyond page plain text |
| Users | person/bot, name/avatar/email, bot workspace limits | Add a `User` object |
| Files | file/file_upload/external; temporary URLs; upload lifecycle | Add file objects and uploads |
| Parent | page/database/data_source/workspace/block/agent | Replace the single page-parent field |
| Icons | emoji/custom_emoji/icon/external/file/file_upload | Replace the glyph string |
| Webhooks | subscriptions, verification token, HMAC-SHA256, event types, retries | Add webhook delivery |
| Limits | rate, size, block and schema limits; 429/529/503 guidance | Add explicit limits and retry policy |
| Versioning | required header, additive changes, opaque cursors, ignore unknown fields | Version our own v2 contract |
| Unfurl | link previews need `title` + `dev`; custom connections closed | Deliberately out of scope |

### Decision

ADR-006 decides to adopt the **patterns** behind our own `/api/v2` contract,
not to clone the public API. The implementation roadmap is specs 030–037.
No private endpoint, capture or proprietary payload was used.

## Current implementation status (2026-11-08)

This is the status of **our** contract, not a claim about the original:

| Area | Status |
| ---- | ------ |
| Workspace/page CRUD, search | Implemented in the loopback API and `Client` |
| Members, shares, sync log | Implemented; persisted in `collab.json` (`spec-019`) |
| Page version history | Implemented (`GET /api/pages/{id}/versions`, 50/page) |
| Export tree | Implemented in memory (`src/collab/export.rs`) |
| Realtime contract | Implemented as data (`GET /api/realtime`) |
| Per-page writes and debounce | Implemented (`spec-020`) |
| Auth/sessions | Decided in ADR-003, **not implemented** |
| WebSocket/CRDT server | Decided, **not implemented** |
| `/api/v1` prefix | Decided, **not served** |
| Pagination/sorting | Not implemented; lists are full snapshots |
| Public API documentation review | **B4 reviewed 2026-11-08**; B5 pending the first v2 slice; B3 blocked |

## What remains unknown

The following cannot be answered from the UI alone and are not answered by
strict clean-room public behavior:

1. Exact original route paths, method names and status codes.
2. Exact authentication/session flow and token lifetimes.
3. Pagination and cursor formats.
4. Sync semantics: what a change contains, how conflicts are resolved, and
   whether the original is CRDT-based.
5. Block-level vs page-level update granularity.
6. Trash semantics (flag vs hard delete vs retention window).
7. Permission inheritance and teamspace/workspace scopes.
8. Export filenames, metadata and CSV columns.
9. Real-time frame format and presence/awareness payloads.
10. Rate limits, retry guidance and error-code taxonomy.

Under the strict scope these remain **Unknown** until public documentation
answers them. They must not be filled in from traffic captures without a new
B1 decision.

> **B4 update (2026-11-08):** the public Notion API docs answer several of these
> at the **integration** level — pagination cursors, error codes, versioning,
> block/property/comment/file shapes and webhook delivery. They do **not** prove
> the original product's private routes or payloads. The public facts are
> recorded in `docs/public-api-analysis.md`; the private behavior remains
> Unknown.

## Compatibility strategy

1. Implement and test the **decided** contract (ADR-003) behind the existing
   `WorkspaceApi` seam.
2. Keep every DTO in `src/collab.rs` and every route in `src/server.rs`.
3. Build a mock server and `tests/contract.rs` from the decided contract.
4. Mark any behavior that is only inferred; do not present it as original
   behavior.
5. If a public document later confirms or contradicts an entry, update this
   file and the relevant ADR/spec before changing code.

## Public sources reviewed in B4 (2026-11-08)

These are public documentation pages, not captures. The full review is
`docs/public-api-analysis.md`.

- Notion Help Center: <https://www.notion.com/help>
- Notion API reference: <https://developers.notion.com/reference/intro>
- Notion API guides: <https://developers.notion.com/docs>
- Export your content: <https://www.notion.com/help/export-your-content>
- Sharing and permissions: <https://www.notion.com/help/sharing-and-permissions>
- Offline mode: <https://www.notion.com/help/offline-mode>
- Version history: <https://www.notion.com/help/version-history>
- Keyboard shortcuts: <https://www.notion.com/help/keyboard-shortcuts>

## Change log for this document

| Date | Change |
| ---- | ------ |
| 2026-11-07 | First inventory under the strict public-behavior-only scope (B1/B2). No captures used. |
| 2026-11-08 | Documentation review (spec-029): requirement range updated to FR-01..FR-77, current implementation status added, B3/B4/B5 status clarified. No new source was observed. |
| 2026-11-08 | **B4 public API review:** reviewed the public Notion developer pages, added the **Observed (public docs)** level, and recorded the evidence summary. No capture, proxy, source map or decompilation. Full analysis: `docs/public-api-analysis.md`; decision: ADR-006; roadmap: specs 030–037. |
