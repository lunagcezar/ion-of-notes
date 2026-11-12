# spec-101 - Skip idle-frame workspace snapshots and diffs

- Status: implemented (2026-11-12)
- Related: `spec-020`, `spec-049`, `spec-097`
- Review source: ChatGPT performance analysis (P0 undo snapshot and diffing)

## Purpose and scope

`NotionApp::ui` deep-cloned the whole workspace every frame to build an undo
snapshot, then deep-compared it at the end. `apply_commands` also diffed the
workspace against `last_committed` every frame. Both happen even when the user
only scrolls an emoji picker or moves the mouse.

This spec gates both costs on a cheap "this frame may mutate" decision:

- a frame may mutate when it has commands, an active pointer, or keyboard focus
  while no overlay is open;
- only such frames clone the pre-frame workspace and compare it for undo;
- `commit_edits` runs only when the frame may have changed the workspace or a
  debounced edit is already pending.

The pure decision is `ui::commands::frame_may_mutate`, tested directly.

Out of scope:

- command-based or inverse-operation undo;
- block-level virtualization;
- async persistence.

## Acceptance criteria

P1. **Given** an idle frame, **when** nothing is clicked, typed or pending,
**then** the workspace is not cloned or diffed.

P2. **Given** an emoji-picker scroll or hover frame, **when** no overlay text
field is focused and no command is produced, **then** no workspace snapshot is
taken.

P3. **Given** a text edit, command, click or drag, **when** the workspace
changes, **then** the pre-frame snapshot is still pushed to the undo stack.

P4. **Given** a pending debounced edit, **when** later idle frames arrive,
**then** the edit still flushes after the debounce window.

P5. **Given** the existing UI/CRDT tests, **when** the gates are applied, **then**
they pass unchanged.

## Test scenarios

- `tests/ui.rs` (`frame_may_mutate`, pending-edit behavior)
- `tests/ui_crdt.rs`
- `tests/fidelity.rs`
- Full quality gate

## Design

- `frame_may_mutate(commands_empty, pointer_active, text_focused, overlay_open)`
  is a pure function so the rule is testable without a window.
- `apply_commands` takes `Option<Workspace>`; `None` means the frame could not
  mutate and the undo comparison is skipped.
- `PendingEdits::is_pending` keeps the debounce flush alive after the
  interaction frame that created the edit.

## Plan / tasks

- [x] Add the pure mutation gate
- [x] Make the frame snapshot optional
- [x] Gate the per-frame diff
- [x] Add tests and run the full gate
