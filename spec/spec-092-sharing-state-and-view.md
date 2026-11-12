# spec-092 - SharingState and SharingView grouping

- Status: implemented (2026-11-12)
- Related: `spec-088`, `spec-023`
- Review sources: Claude and ChatGPT (composable app state, narrow view models)

## Purpose and scope

Sharing and invite state was spread across `NotionApp` as eight `share_*` /
`invite_*` fields and repeated in `ViewState`. This spec groups them into
`SharingState` (runtime) and `SharingView` (presentation snapshot).

Scope:

- `SharingState`: open, members, invite input/role/message, invite error/status
  and general access;
- `SharingView`: the presentation fields;
- `ViewState` holds `sharing: SharingView`;
- references use `self.sharing.*` / `view.sharing.*`.

Out of scope:

- collaboration, settings and AI state grouping;
- changing the Share popover or invite behavior.

## Acceptance criteria

P1. **Given** the Share popover, **when** it opens, **then** members, general
access, invite input and status are unchanged.

P2. **Given** an invite submit, **when** it succeeds or fails, **then** the
error/status messages are unchanged.

P3. **Given** the shared page/list views, **when** they render, **then** they
read the grouped `SharingView`.

P4. **Given** the existing sharing/client/UI tests, **when** the fields move,
**then** they pass unchanged.

## Test scenarios

- `tests/ui.rs`
- `tests/client.rs`
- `tests/comments.rs` (members/users)
- `tests/screenshot.rs`
- Full quality gate

## Design

- `SharingState` owns the invite role/message that only the app needs.
- `SharingView` owns only immutable presentation fields.
- No new trait or state framework.

## Plan / tasks

- [x] Group the runtime and view fields
- [x] Update references
- [x] Run the full gate and commit
