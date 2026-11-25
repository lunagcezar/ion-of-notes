# spec-146 - Patch-based undo

- Status: implemented (2026-11-25)
- Related: `spec-020`, `spec-141`, `docs/performance-roadmap.md`
- Review source: 2026-11-25 ChatGPT latency review, item 3.6

## Purpose and scope

A typing burst still cloned the whole workspace before rendering because undo
kept `Vec<Workspace>` snapshots. Typing only changes the current page, so this
spec records a page patch for pure text frames and keeps full snapshots for
clicks, overlays and keyboard commands.

### Capture decision (before rendering)

- Full snapshot: pointer activity, open overlay, or a shortcut command already
  collected for the frame.
- Page patch: a focused text field with text/key events and nothing else.
- Snapshots remain grouped to one per 0.8 s of continuous mutation (spec-141).

### Entry

`UndoEntry` has two shapes:

- `Snapshot(Workspace)` for structural/navigation changes;
- `Page { page_id, before, after }` for text frames.

Undo applies the `before` side; redo applies the `after` side. The entry is moved
between the undo and redo stacks, so no workspace clone is made on undo/redo.
The block index is refreshed after either direction.

Out of scope: a command-by-command patch log for every mutation; page patches
cover the hot typing path, everything else keeps the full snapshot.

## Acceptance criteria

P1. **Given** a page-only edit, **then** `UndoEntry::from_capture` returns a
page patch and no workspace copy.

P2. **Given** undo, **then** only the edited page is restored and redo re-applies
it without cloning the workspace.

P3. **Given** an unchanged page, **then** no entry is produced.

P4. **Given** a structural command or pointer interaction, **then** a full
snapshot is still recorded.

## Test scenarios

- `tests/ui.rs` (`page_patch_undo_restores_only_the_edited_page`,
  `unchanged_page_patch_produces_no_undo_entry`).
- Existing frame/commit and UI suites.

## Plan / tasks

- [x] Add `UndoCapture`/`UndoEntry` and the block-index refresh.
- [x] Capture page patches for pure text frames.
- [x] Move entries between the stacks instead of cloning.
- [x] Add the tests.
