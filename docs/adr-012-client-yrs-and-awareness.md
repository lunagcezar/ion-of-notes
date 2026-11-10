# ADR-012 — Client-side Yrs and awareness

- **Status:** Accepted (2026-11-10)
- **Related:** ADR-009 (realtime), ADR-011 (server Yrs), spec-043

## Context

The server merges Yrs text (spec-042), but the egui editor still writes whole
`Block.text` values. The desktop also runs the server in-process, so a remote
WebSocket client is not required to exercise the collaboration path.

## Decision

1. **Client-side `CrdtSession`.** The UI owns a Yrs `Doc` for the open page and
   derives a minimal prefix/suffix diff from the last known block text before
   applying insert/delete operations.
2. **In-process transport first.** `WorkspaceApi` gains default CRDT/realtime/
   awareness methods; `WorkspaceServer` overrides them. The desktop uses these
   directly, so no Tokio runtime or socket is needed in the UI.
3. **REST debounce stays out of the way.** A successful local CRDT edit updates
   the page's committed block text, so the normal edit queue does not send a
   whole-page PATCH that would reset the server CRDT document.
4. **Awareness is per session.** The hub stores `Awareness { session_id,
   member_id, name, color, page_id, block_id, index, selection_end }` and
   broadcasts the list. The local session filters its own entry.
5. **Cursor overlay is a presence cue.** The editor measures the text prefix
   with the editor font and draws a caret line, name label and optional
   selection tint. Multi-line positions use the first-line width; a precise
   galley-based caret is a later refinement.

## Alternatives considered

| Alternative | Why not now |
| ----------- | ----------- |
| Remote WebSocket client immediately | More moving parts (runtime/thread, reconnect, auth); the in-process transport proves the CRDT/awareness path first. |
| CRDT for the whole page structure | Too large; text is the collaboration need. |
| Replace `TextEdit` with a custom editor | Required for pixel-perfect cursors, but a much larger UI rewrite. |
| Send REST PATCH for local text | Resets the server CRDT document and loses concurrent edits. |
| Awareness via presence only | Presence has no cursor/selection position. |

## Consequences

Positive:

- The desktop editor participates in Yrs merge without a network server.
- Remote carets and selections are visible.
- The REST debounce and CRDT path do not fight each other.
- The transport methods are defaulted, so test stubs and future remote clients
  remain compatible.

Costs:

- Title edits are still REST-owned.
- The cursor overlay is approximate on multi-line text.
- Awareness is in-memory and not persisted.
- A remote WebSocket adapter still needs reconnect/backoff and auth handling.

## Follow-up

- Add a remote WebSocket transport behind the same client methods.
- Add title CRDT and precise galley-based cursors.
- Add typing indicators, follow mode and awareness expiry.
