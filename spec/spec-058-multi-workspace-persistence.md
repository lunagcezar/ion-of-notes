# spec-058 - Multi-workspace persistence

- Status: implemented for the desktop catalog (2026-11-11); server API follow-up
- Related: `spec-011`, `spec-039`, `spec-041`, `spec-050`, `spec-057`
- Reference: `reference/workspace-switcher.png`, `reference/new-workspace.png`
- Contract docs: `docs/data-model.md`, `docs/api-contract.md`,
  `docs/adr-003-self-hosted-collaborative-api.md`

## Purpose and scope

Today the application owns exactly one `Workspace`. The workspace switcher
shows a single "Local workspace" row and "Create new workspace" opens the
purpose dialog, but the chosen purpose only changes the current workspace
(`spec-057`, implementation note). This spec adds a real persisted catalog of
workspaces and makes create, rename, delete and switch work end to end.

The catalog is the source of truth for the active workspace. Existing page,
search, share, realtime and CRDT routes keep working against the active
workspace, so the change is additive at the API surface.

Scope:

- A persisted `WorkspaceCatalog` with one active workspace and any number of
  additional workspaces.
- Create from a name, icon and starter purpose; rename; delete; activate.
- Per-workspace pages, settings, shares, comments, files, version history and
  collaboration state.
- JSON and SQLite adapters, a migration from the current single
  `workspace.json`, and the same behavior in remote mode.
- The workspace switcher, create dialog and management actions in the UI.

Out of scope:

- Moving pages between workspaces.
- Cross-workspace search or a combined Library.
- Per-workspace accounts; accounts remain server-global.
- Billing, quotas or workspace-level domains.

## Evidence

The review after `spec-057` reported:

1. The switcher lists only one workspace.
2. "Create new workspace" opens the purpose dialog but does not create or
   switch to a second workspace.
3. There is no persisted place to store more than one workspace, and no API to
   list or activate them.
4. The SQLite adapter stores a single workspace snapshot.

Current code seams:

| Area | Code |
| ---- | ---- |
| Domain workspace | `src/model/workspace.rs`, `src/model/settings.rs` |
| Store ports | `src/store.rs` (`WorkspaceStore`, `CollabStore`) |
| SQLite adapter | `src/store/sqlite.rs` |
| Server state and routes | `src/server.rs`, `src/server/routes.rs`, `src/server/routes/v2.rs` |
| HTTP/WebSocket adapter | `src/server/web.rs` |
| Remote client | `src/remote.rs`, `src/client.rs` |
| Composition root | `src/app.rs` |
| Switcher and dialogs | `src/ui/dialogs/menus.rs`, `src/ui/dialogs/onboarding.rs` |
| Workspace commands | `src/ui/apply/workspace.rs`, `src/ui/commands.rs` |

## Data model

```text
WorkspaceCatalog
  active_id: String
  entries: Vec<WorkspaceEntry>

WorkspaceEntry
  workspace: Workspace          // pages, settings, comments, files, versions
  collab: CollabState           // members, shares, sync log, webhooks
  created_at: String
  updated_at: String
```

`WorkspaceSummary` is the list DTO:

```text
WorkspaceSummary
  id: String
  name: String
  icon: String
  active: bool
  page_count: usize
  updated_at: String
```

Rules:

- At least one workspace always exists. Deleting the last workspace is refused
  with `409`.
- Deleting the active workspace activates the next workspace in catalog order.
- Creating a workspace activates it.
- Workspace ids are stable UUIDs. The migrated workspace keeps the current
  `local-workspace` id when possible.
- Accounts (`AuthStore`) stay server-global. Shares, members and webhooks are
  stored per catalog entry.

## API contract

All routes use the v2 envelope and the existing version/auth middleware.

| Method | Path | Purpose |
| ------ | ---- | ------- |
| `GET` | `/api/workspaces` | List `WorkspaceSummary` values |
| `POST` | `/api/workspaces` | Create `{ name, icon, purpose }` and activate it |
| `POST` | `/api/workspaces/{id}/activate` | Switch the active workspace |
| `PATCH` | `/api/workspaces/{id}` | Rename or change the icon |
| `DELETE` | `/api/workspaces/{id}` | Delete a non-active or active workspace |

Errors:

- `404` unknown workspace.
- `409` deleting the last workspace, or creating a duplicate name if the
  duplicate-name rule is enabled.
- `400` empty name.
- `422` invalid icon token.

Existing `/api/workspace`, `/api/pages`, `/api/search`, `/api/shares`,
`/api/sync`, `/api/realtime` and `/api/crdt` routes resolve the active
workspace at request time. A switch invalidates the client cache and the
realtime subscription.

## Acceptance criteria

### Catalog and persistence

W1. **Given** a fresh install, **when** the server starts, **then** a catalog
with one active workspace is created from the seed.

W2. **Given** an existing single `workspace.json`, **when** the server starts
with the new version, **then** the file is migrated into a catalog with one
entry, the original pages/settings are preserved, and the old file is kept as a
backup.

W3. **Given** several workspaces, **when** the server restarts, **then** all
workspaces, the active id and each workspace's pages/settings/collab state are
unchanged.

W4. **Given** SQLite persistence, **when** workspaces are created, renamed,
activated and deleted, **then** a restart produces the same catalog.

### API and client

W5. **Given** the catalog API, **when** a workspace is created, **then** the
response is `201` with its summary and the active workspace changes.

W6. **Given** a request for pages after an activate call, **when** the active
workspace changed, **then** the pages come from the newly active workspace.

W7. **Given** the last workspace, **when** delete is requested, **then** the
server returns `409` and the catalog is unchanged.

W8. **Given** the remote client, **when** it lists/creates/switches/renames/
deletes workspaces, **then** the same operations work over HTTP and WebSocket
reconnection follows the active workspace.

### UI

W9. **Given** the workspace switcher, **when** it opens, **then** it lists every
workspace with the active one checked, plus "Create new workspace".

W10. **Given** "Create new workspace", **when** the dialog is submitted with a
name/icon/purpose, **then** the workspace is created, activated and its pages
are shown in the tabs and sidebar.

W11. **Given** a non-active workspace, **when** it is selected, **then** the
active page, tabs, history, search cache, realtime session and CRDT session are
reset to the new workspace.

W12. **Given** the active workspace, **when** rename/delete is used, **then**
the switcher and window title update and the last workspace cannot be deleted.

## Test scenarios (written before implementation)

- **Store:** catalog round-trip in JSON; SQLite round-trip; restart; migration
  from `workspace.json`; delete-active activates the next entry; last-delete
  refusal.
- **Server:** `GET/POST/PATCH/DELETE` status codes and envelopes; activate then
  fetch pages; v1 compatibility routes; v2 capabilities include `workspaces`.
- **Client/remote:** list/create/switch/rename/delete over the Axum adapter;
  realtime resubscribe after a switch.
- **UI:** preview states `workspace-list`, `workspace-create`,
  `workspace-rename`, `workspace-delete`; switcher ordering; active check.
- **Regression:** all existing page/share/realtime/CRDT tests, the full
  two-pass gate and a screenshot of the switcher with three workspaces.

## Design

- Add `WorkspaceCatalog` to `model` with catalog-level validation and a
  `WorkspaceSummary` DTO. Keep `Workspace` unchanged so page logic is not
  rewritten.
- Replace the single-workspace slot in `WorkspaceServer` with a catalog slot.
  The active workspace is resolved through one helper used by every route.
- Extend `WorkspaceStore` with catalog operations rather than adding a second
  unrelated port: `load_catalog`, `save_catalog`, `save_entry`, `delete_entry`.
  The JSON adapter writes `workspaces.json` plus an optional
  `workspaces/<id>.json` per entry; the SQLite adapter adds `workspaces` and
  `workspace_entries` tables.
- The catalog mutation methods live in one use-case module, not in the route
  handlers. Routes only adapt DTOs.
- The remote client gets `list_workspaces`, `create_workspace`,
  `activate_workspace`, `rename_workspace`, `delete_workspace`.
- The UI keeps the current switcher shape and adds a management row per
  workspace (rename/delete) plus the create dialog fields.
- On switch, clear `tabs`, `nav_history`, `search_cache`, `crdt`,
  `remote_awareness` and the pending edit queue, then reload the workspace.

## Plan / tasks

- [x] Model: catalog, summaries, validation and migration helper.
- [x] Store: JSON catalog adapter and migration test (SQLite catalog follow-up).
- [ ] Server: catalog use cases, routes, v2 capabilities and active resolution (follow-up).
- [x] Client: switch uses the existing replace-workspace path and resets UI/realtime state.
- [x] UI: switcher list, create/rename/delete actions and reset-on-switch.
- [x] Tests: catalog model/store round-trip and UI preview states.
- [x] Docs: changelog and spec index (API contract follow-up).
- [x] Full gate and screenshot comparison.

## Implementation note\n\nThe desktop catalog is implemented end to end: `workspaces.json`, create,\nswitch, rename, delete, migration from the single workspace, and the switcher\nUI. The server still owns one active workspace; `/api/workspaces` and remote\nmulti-workspace switching are a follow-up because they require a server-side\ncatalog and a v2 contract change. The UI uses the existing replace-workspace\npath to move the active snapshot, so pages, settings and shares are preserved.\n\n## Decisions and open questions

- Decision: accounts are server-global; workspace membership is represented by
  the existing share/member data inside each catalog entry.
- Decision: the last workspace cannot be deleted; the UI disables the action.
- Open: whether a workspace name must be unique. The default is "warn but
  allow duplicates"; if the person prefers unique names, add a `409` on create.
- Open: whether the purpose dialog should also seed a template when the chosen
  purpose is "skip". The default is to seed an empty workspace.
