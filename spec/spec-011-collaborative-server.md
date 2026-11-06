# spec-011 — Self-hosted client/server with a decided collaborative API

## Purpose and scope

The person changed the direction of the reconstruction: from a single-user,
local-only desktop app to a **client/server architecture** with a
**self-hosted server** that has **accounts**, **synchronization of edited
documents**, **sharing**, and **real-time editing**. They explicitly asked to
**decide the API to use**, to keep **DRY / Clean Code / SOLID /
modularization**, and to take into account the workspace **export** format
(markdown page tree, "folder" pages linking to children, and a `People`
folder that records membership/access).

This spec covers the first implemented slice of that decision:

- A new, cohesive `collab` domain module for accounts (`Member`, `Role`,
  `MemberDraft`), permissions (`Share`, `Access`), synchronization
  (`Change`, `ChangeOp`, `SyncBatch`), real-time DTOs (`RealtimeEvent`,
  `RealtimeKind`, `RealtimeContract`) and export DTOs (`ExportFile`).
- A `CollabState` owned by the server, with invite / access resolution /
  change-log methods.
- New `WorkspaceApi` routes (`/api/members`, `/api/shares`, `/api/sync`,
  `/api/export`, `/api/realtime`) plus typed `Client` methods — the concrete,
  testable form of the decided API (the full contract is in
  `docs/adr-003-self-hosted-collaborative-api.md`).
- A workspace **export tree** that reproduces the described structure:
  one markdown file per page, folder pages that link to their children, a
  `People` folder with membership files and a `_all.csv`.
- **De-emphasising "free & open for all"** in the window: the app is open
  source (MIT) as a license fact, not a marketing feature. The sidebar card
  becomes "Invite members", Settings gets a neutral "Workspace & sync"
  section, and the demo callout describes the self-hosted, syncing,
  collaborative server instead of "no account, no data leaves this device".

Out of scope for this slice (decided but deferred behind the seams, per
ADR-003): a real Axum/PostgreSQL server, password hashing and Bearer-token
auth middleware, a running WebSocket server and a Yrs CRDT merge, and a
`CollabStore` persistence adapter. The domain and the API **contract** for
these are defined now so they can be added without rewriting the core.

## Evidence (what the reference pictures and the export show)

- `reference/picture-1`: an **"Invite members / Collaborate with your team"**
  card at the bottom of the sidebar; a signed-in account row.
- `reference/picture-12`: the **Share** popover — invite by email, "Full
  access", "People invited … 2 people", "General access / Everyone at
  <workspace>", "Copy link". → accounts, roles, permissions.
- `reference/picture-27`: the **People** settings page — "Manage people in
  your workspace and their roles", "Members 1 / Guests 2", "Workspace owner".
- `reference/picture-24`: the **Export** settings area — "Workspace content"
  and "Members … as CSV". → the markdown tree and the People CSV.
- The supplied workspace export: a tree of `Title <uuid>.md` files; a folder's
  markdown lists links to its sub-folders/sub-files; a `People` folder whose
  markdown defines `filters:`/`sort:`, a `People …_all.csv`, and one person
  file per member with `Membership Type` and `Person` fields.

## Acceptance criteria (Given / When / Then)

1. **Given** a fresh server, **when** its collaboration state is read, **then**
   there is exactly one `Owner`, general "everyone" full access, and an empty
   change log.
2. **Given** a `MemberDraft`, **when** a member is invited, **then** a `Member`
   is created and appears in `GET /api/members` / `list_members`.
3. **Given** a `Share` for a page and subject, **when** access is resolved,
   **then** the most specific grant wins and `can_write` reflects
   `Full`/`Edit` vs `View`/`None`; upserting never duplicates a grant.
4. **Given** a document mutation, **when** it is committed, **then** one
   `Change` is appended; `GET /api/sync?since=<cursor>` returns only changes
   after the cursor and an empty batch once caught up.
5. **Given** a workspace with nested pages, **when** `GET /api/export` is
   called, **then** the tree contains a root folder page that links to
   top-level pages, folder pages that link to their children, and a `People`
   folder with `People.md`, `People_all.csv`, and one file per member whose
   content contains `Membership Type: Workspace owner`.
6. **Given** the decided real-time contract, **when** `GET /api/realtime`
   is called, **then** it describes a WebSocket channel and CRDT conflict
   resolution.
7. **Given** the app window, **when** rendered, **then** the sidebar shows an
   "Invite members" card (not "Free & open for all"), Settings shows a
   "Workspace & sync" section (not a plan/feature-tier list), and the demo
   callout describes the self-hosted syncing server. The 29 reference
   pictures remain the visual source of truth.
8. **Given** an empty member name/email, **when** `POST /api/members` is
   called, **then** the server returns `400` and adds no member.

## Test scenarios (written before implementation)

- `roles_and_access_have_stable_labels`
- `seeded_state_has_one_owner_and_general_full_access`
- `invite_adds_a_member_and_access_resolves_most_specific_first`
- `sync_log_returns_only_changes_after_the_cursor`
- `page_markdown_renders_block_kinds_and_table_rows`
- `people_files_record_membership_and_view_definition`
- `export_tree_has_folder_pages_child_links_and_a_people_folder`
- `realtime_contract_describes_a_websocket_crdt_channel`
- `collaborative_api_roundtrip_through_the_client`
- `empty_member_draft_is_rejected_by_the_server`

All ten live in `tests/collab.rs` (one integration-test crate per module),
use only the public API, and were observed to fail (red) before
`src/collab.rs` and the routes existed.

## Plan

1. Write `tests/collab.rs` first and observe it fail to compile (red).
2. Add `src/collab.rs` (domain + export) and re-export it from `lib.rs`.
3. Add `CollabState` to `WorkspaceServer`, route every mutation through one
   `write(op, page_id)` path that also appends a `Change` (DRY / SRP).
4. Add the new `WorkspaceApi` routes and typed `Client` methods.
5. Remove the `edition` module and the "free & open for all" UI; reframe the
   sidebar card, Settings section and demo callout for the self-hosted server.
6. Decide and record the API in `docs/adr-003-*`; update `AGENTS.md`,
   `docs/requirements.md`, `docs/data-model.md`, `docs/unknowns.md`,
   `docs/adr-001-*`, `README.md`, `LICENSE`, `CHANGELOG.md` and retire
   `spec/spec-007`.
7. `cargo build`, `cargo test`, and `--screenshot`; compare the fresh PNG
   with all 29 reference pictures.

## Notes

- No code, assets, or text are taken from the original program (clean room);
  names in the demo `People` folder are neutral placeholders.
- DRY: one `write` path, one JSON envelope helper, one real-time contract,
  shared test helpers; SOLID: `collab` has one responsibility, new transports
  are added as adapters (OCP/DIP), public interfaces stay small.
