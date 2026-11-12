# spec-093 - CollaborationState grouping

- Status: implemented (2026-11-12)
- Related: `spec-088`, `spec-092`, `spec-040`, `spec-043`
- Review sources: Claude and ChatGPT (composable app state, one cohesive state owner)

## Purpose and scope

CRDT, realtime and awareness state was spread across seven flat `NotionApp`
fields (`crdt`, `crdt_page`, `crdt_rx`, `remote_awareness`,
`awareness_session`, `awareness_color`, `last_awareness`). This spec groups
them into one `CollaborationState` value so the collaboration concern has a
single owner and the app struct stays readable.

Scope:

- `CollaborationState`: CRDT session, open page, realtime receiver, remote
  awareness, awareness session, awareness color and last broadcast awareness;
- `NotionApp` holds `collaboration: CollaborationState`;
- references use `self.collaboration.*`.

Out of scope:

- changing the CRDT protocol, projection or awareness behavior;
- grouping settings and AI state (next slice);
- narrowing `ViewState`.

## Acceptance criteria

P1. **Given** a page switch, **when** the CRDT session syncs, **then** the
open page, local session reset and projection are unchanged.

P2. **Given** a realtime broadcast, **when** a remote update or awareness
message arrives, **then** it is applied exactly as before.

P3. **Given** a reconnect, **when** the session resets, **then** the CRDT
session, receiver and awareness are re-created as before.

P4. **Given** the existing UI/CRDT tests, **when** the fields move, **then**
they pass unchanged.

## Test scenarios

- `tests/ui.rs`
- `tests/ui_crdt.rs`
- `tests/crdt.rs`
- `tests/screenshot.rs`
- Full quality gate

## Design

- One struct with public fields, matching the existing `SearchState` /
  `SharingState` convention.
- No behavior moves: only field paths change.
- The struct is the seam for a later owned collaboration session object.

## Plan / tasks

- [x] Add `CollaborationState`
- [x] Update the field references
- [x] Run the full gate and commit
