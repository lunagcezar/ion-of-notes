# spec-020 — Per-page writes and debounced persistence

## Purpose and scope

The UI currently sends `PUT /api/workspace` after any workspace change. That
replaces the whole document for a single keystroke, records versions for every
changed page and scales with workspace size. This spec moves single-page edits
to `PATCH /api/pages/{id}`, keeps the full snapshot only for structural
multi-page changes, and debounces persistence while typing.

Scope:

- a pure `PageDiff` (changed / added / removed / reordered pages);
- a pure `CommitPlan` decision (none / patch pages / full replace);
- a pure `PendingEdits` debounce helper;
- UI integration: flush on page/tab switch, reload, reset and close;
- tests for the pure helpers and the existing client/server path.

Out of scope:

- collaborative CRDT/real-time deltas;
- block-level endpoints;
- server-side request coalescing;
- changing the REST contract beyond using the existing `PATCH`.

## Evidence

- `src/ui/apply.rs`: `self.client.replace_workspace(&self.workspace)` runs
  after any change.
- `src/server.rs`: `PUT /api/workspace` records a version for every changed
  page and persists the whole document.
- `src/client.rs` already exposes `update_page` (`PATCH /api/pages/{id}`).
- `spec/spec-016-architecture-follow-up-and-observation.md` A2 plans this.
- NFR-15: an unchanged frame must not call the server; a single-page edit must
  not send a whole-workspace payload.

## Acceptance criteria

1. **Given** an edit to one page, **when** the debounce elapses, **then** only
   that page is sent with `PATCH /api/pages/{id}`.
2. **Given** edits to several pages, **when** the debounce elapses, **then**
   each changed page is patched; no full snapshot is sent.
3. **Given** a page added, removed or reordered, **when** the debounce elapses,
   **then** the full `PUT /api/workspace` fallback is used.
4. **Given** rapid typing, **when** the user pauses for the debounce window,
   **then** at most one persistence batch is sent for the settled state.
5. **Given** a page/tab switch, reload, reset or window close, **when** it
   happens, **then** pending edits are flushed first.
6. **Given** no workspace change, **when** a frame redraws, **then** no
   persistence request is sent.

## Test scenarios (written before implementation)

- `tests/model.rs`: `Workspace::diff` detects changed, added, removed and
  reordered pages; identical workspaces produce an empty diff.
- `tests/ui.rs`: `plan_for` returns `None`, `Update(ids)` or `Replace`;
  `PendingEdits` respects the debounce window, force flush and reset.
- Existing `tests/client.rs` and `tests/server.rs` stay green.
- `cargo test --all-targets`, clippy and the screenshot stay green/unchanged.

## Design

- `model::PageDiff` (in `src/model/diff.rs`):
  - `changed: Vec<String>`, `added`, `removed`, `reordered: bool`;
  - `is_empty()` and `requires_full_replace()`.
- `ui::edit_queue`:
  - `CommitPlan::{None, Update(Vec<String>), Replace}`;
  - `plan_for(&PageDiff)` decides the request shape;
  - `PendingEdits { last_change, debounce, force }` decides when to flush.
- `NotionApp` keeps `last_committed` as the last successfully sent state.
  `commit_edits(ctx, force)`:
  1. computes `last_committed.diff(&workspace)`;
  2. marks the debounce when non-empty;
  3. flushes when forced or the debounce elapsed;
  4. sends one `update_page` per changed id, or `replace_workspace` when the
     diff requires it;
  5. updates `last_committed` only after a successful send.
- Force flush on page/tab switch, library/settings open, reload, new/delete
  page, reset, restore, version history open and window close.
- Debounce window: 250 ms.

## Plan / tasks

- [x] Spec + failing tests (red)
- [x] `model::PageDiff` + tests
- [x] `ui::edit_queue` (`CommitPlan`, `PendingEdits`) + tests
- [x] `NotionApp` integration and force-flush points
- [x] Docs, changelog, build/test/clippy/screenshot, commit
