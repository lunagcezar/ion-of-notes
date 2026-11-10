# spec-043 — Client-side Yrs and awareness cursors

## Purpose and scope

The server merges Yrs text, but the desktop editor still writes whole
`Block.text` values through REST. This spec adds a client-side Yrs session for
the open page, sends compact updates through the in-process transport, projects
remote merges back into the editor, and adds awareness messages with cursor and
selection positions plus a visible remote-caret overlay.

In scope:

- `CrdtSession` in `ui/crdt.rs`: seed from the page, local diff, remote apply,
  project back, state vector;
- `WorkspaceApi`/`Client` in-process methods for CRDT sync/update, realtime
  subscription and awareness;
- `Awareness` messages in the realtime hub and WebSocket protocol;
- editor wiring: text changes become CRDT updates; cursor changes become
  awareness; remote carets/selections render in the block;
- tests for local diff, convergence and awareness broadcast.

Out of scope:

- a remote WebSocket transport for the desktop (the in-process transport is
  used today; the WS client adapter is a later spec);
- pixel-perfect multi-line caret positioning (the overlay uses first-line
  width; it is a presence cue);
- awareness persistence, typing indicators, follow mode or remote selection
  colors per member beyond the supplied color;
- CRDT for title edits (title remains REST-owned for now).

## Evidence

- `src/server/crdt.rs` — server Yrs merge (spec-042).
- `src/server/realtime.rs` — WebSocket messages and hub.
- `src/ui/render/content.rs` — `TextEdit` block editor.
- `src/ui/app.rs`, `src/ui/apply.rs` — app state and commands.
- `src/client.rs` — the UI's only data access.

## Acceptance criteria

1. **Given** a local block edit, **when** the text changes, **then** the client
   session emits a Yrs update and the server merges it.
2. **Given** a remote CRDT update, **when** it arrives, **then** the client
   applies it and projects the merged text into the open page.
3. **Given** two client sessions, **when** they exchange updates, **then** their
   projected text converges.
4. **Given** a focused editor, **when** the caret or selection moves, **then**
   an awareness message is published.
5. **Given** two sessions, **when** one publishes awareness, **then** the other
   receives the cursor/selection and the editor draws a colored caret/selection
   and name.
6. **Given** the local session, **when** its own awareness is broadcast,
   **then** it is filtered out of the remote overlay.
7. **Given** the existing suite, **when** it runs, **then** all tests stay green
   and the new `tests/ui_crdt.rs` and awareness test pass.

## Test scenarios (written before implementation)

- `tests/ui_crdt.rs` (new):
  - local edit emits an update and projects back;
  - two sessions converge;
  - unchanged text emits no update.
- `tests/realtime.rs`: hub awareness broadcast and self/session fields.
- Existing `tests/ui.rs`, `tests/app.rs`, `tests/crdt.rs` stay green.

## Design

```text
TextEdit changed -> Command::CrdtTextChanged
   -> CrdtSession::local_text (Yrs diff) -> Client::apply_crdt_update
   -> server merge -> broadcast CrdtUpdate
   -> CrdtSession::apply_update -> project page -> editor

TextEdit cursor -> Command::CrdtCursor
   -> Client::publish_awareness -> RealtimeHub
   -> broadcast Awareness -> ViewState.remote_cursors
   -> draw_remote_cursors (line + name + selection)
```

- `CrdtSession` keeps the last known text per block and derives a minimal
  prefix/suffix diff before applying insert/delete to the Yrs text.
- The UI still owns `Block.text`; the session only bridges it to Yrs.
- Local CRDT edits mark the page's committed block text so the normal REST
  debounce does not send a whole-page PATCH and reset the server document.
- `WorkspaceApi` gains default real-time methods; `WorkspaceServer` overrides
  them for the in-process desktop. Remote transports can override them later.
- Awareness is stored per WebSocket session in the hub; the sender's own
  session is filtered from the local overlay.
- The overlay measures the text prefix with the editor font and draws a caret
  line, a name label and an optional selection tint.

## Plan / tasks

- [x] Spec + failing tests (red).
- [x] Awareness type, hub storage and WebSocket message.
- [x] WorkspaceApi/Client real-time methods.
- [x] `CrdtSession` local diff/apply/project.
- [x] Editor commands, app state, receiver drain and projection.
- [x] Remote caret/selection overlay.
- [x] Tests and docs.
- [x] Build/test/clippy/fmt/docs/screenshot, commit.

## Result

- Added `src/ui/crdt.rs`: `CrdtSession` seeds from the page, derives a minimal
  text diff, applies remote updates, projects merged text and exposes a state
  vector.
- Added default CRDT/realtime/awareness methods to `WorkspaceApi`;
  `WorkspaceServer` overrides them for the in-process desktop and `Client`
  exposes typed wrappers.
- Added `Awareness` and hub storage; WebSocket `awareness` messages and
  `ServerMessage::Awareness` broadcast.
- The editor pushes `CrdtTextChanged` and `CrdtCursor`; the app drains the
  realtime receiver, applies updates, projects pages and filters its own
  awareness.
- `draw_remote_cursors` renders a colored caret, name and optional selection
  tint using the editor font.
- Local CRDT edits mark the committed block text so the REST debounce does not
  reset the server document.
- Tests: 4 in `tests/ui_crdt.rs` plus a hub-awareness test; the suite is 248
  tests.
- Added `docs/adr-012-client-yrs-and-awareness.md`.

## Requirement mapping

| Requirement | How |
| ----------- | --- |
| FR (client CRDT) | `CrdtSession` diff/apply/project |
| FR (awareness) | cursor/selection messages and overlay |
| NFR (no REST reset) | committed block text updated on local CRDT edits |
| NFR (compatibility) | default WorkspaceApi methods keep test stubs working |
