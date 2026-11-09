# spec-025 — Settings pages

## Purpose and scope

The Settings window had placeholder pages: General showed read-only rows,
Sidebar listed labels without toggles, People was a single invite button, and
Offline only listed pages. This spec makes the General / Sidebar / People /
Offline pages functional and persists the workspace-level settings.

Scope:

- a persisted `WorkspaceSettings` value on `Workspace`:
  - workspace id/icon, landing page, allowed email domains;
  - sidebar visibility toggles and "show other apps";
  - analytics, offline automatic downloads;
  - people directory / recent activity / hover cards;
- editable General page (name, icon, landing page, domains, exports);
- Sidebar page with working toggles, exports and analytics;
- People page with the member directory, toggles, danger-zone placeholder and
  workspace ID;
- Offline page with automatic downloads, search and last-edited times;
- workspace-level changes use the full `PUT /api/workspace` fallback;
- preview hooks for each settings page.

Out of scope:

- actually deleting a workspace (the danger-zone button is intentionally
  disabled in this build);
- real AI Meeting Notes / Agents / Templates data;
- email delivery for invites;
- server-side settings validation beyond serde defaults.

## Evidence

- `screenshots/picture-51.png`: Offline settings with automatic downloads and
  an offline-pages table.
- `screenshots/picture-64.png`: Language & time page.
- `screenshots/picture-70.png`: General settings.
- `screenshots/picture-71.png`: Sidebar settings and export.
- `screenshots/picture-72.png`: People settings and danger zone.
- `src/ui/dialogs/settings_pages.rs`: previous placeholder pages.
- `src/client.rs`: `export_tree`, `list_members`.

## Acceptance criteria

1. **Given** the General page, **when** the user edits the workspace name,
   icon, landing page or allowed domains, **then** the values persist in the
   workspace document.
2. **Given** a workspace-level setting change, **when** the debounce elapses,
   **then** the full `PUT /api/workspace` fallback is used (not a page PATCH).
3. **Given** the Sidebar page, **when** a visibility toggle is clicked, **then**
   the persisted setting changes; the export buttons report how many files the
   client returned.
4. **Given** the People page, **when** it opens, **then** the member directory
   is loaded and listed; the directory/activity/hover toggles persist.
5. **Given** the Offline page, **when** it renders, **then** automatic downloads
   can be toggled and offline pages can be searched and show their last-edited
   time.
6. **Given** `ION_OF_NOTES_SHOW=settings-general|settings-sidebar|settings-people|settings-offline`,
   **when** the screenshot runs, **then** the corresponding page is rendered.

## Test scenarios (written before implementation)

- `tests/model.rs`: `WorkspaceSettings` JSON round-trip; a settings/name change
  marks `PageDiff.settings_changed` and requires a full replace.
- Existing client tests cover member loading and export.
- The four settings screenshots are checked manually.

## Design

- `model/settings.rs`: `WorkspaceSettings` with serde defaults; `Workspace`
  gains `#[serde(default)] settings`.
- `PageDiff` gains `settings_changed`; `Workspace::diff` compares `name` and
  `settings`; `requires_full_replace` includes it.
- `render_settings` receives `&mut Workspace`; the General/Sidebar/People/
  Offline pages mutate the settings directly.
- `Command::ExportWorkspace`, `Command::ExportMembers` and
  `Command::SetSettingsStatus` surface export results/status.
- `OpenSettings`/`SetSettingsPage("People")` refresh the member directory.
- `ION_OF_NOTES_SHOW` accepts the four page names for screenshots.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `WorkspaceSettings` + workspace diff/persistence
- [x] General / Sidebar / People / Offline pages
- [x] Export/status commands and member refresh
- [x] Docs, changelog, build/test/clippy/fmt/screenshots, commit
