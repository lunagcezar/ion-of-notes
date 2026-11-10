# ADR-011 — Yrs CRDT text merge

- **Status:** Accepted (2026-11-10)
- **Related:** ADR-009 (change-feed first), spec-040 (WebSocket),
  spec-042 (Yrs CRDT)

## Context

The WebSocket channel streams page-level changes and presence. Two clients
editing the same block still depend on whole-page writes and `If-Match`, so
concurrent text edits are not merged. The page/block structure is REST-owned
and the durable snapshot is the source of truth.

## Decision

1. **Yrs per page.** One `yrs::Doc` per page holds a `"title"` text and one
   `"block:<id>"` text per block (nested blocks included).
2. **Structure stays REST-owned.** CRDT only merges text; block creation,
   deletion, properties and databases remain REST operations.
3. **State-vector sync.** `crdt_sync(page, state_vector?)` returns an update
   plus the current state vector. Clients send `crdt_update` with base64 Yrs
   updates.
4. **Projection is durable.** After each applied update the server writes the
   merged title/block text into the workspace page and persists it. CRDT
   history is in memory; the page snapshot re-seeds after a restart.
5. **REST resets CRDT.** A REST page write drops that page's CRDT document so
   the next sync re-seeds from the new snapshot. This avoids two competing
   sources of truth.
6. **WebSocket transport.** `crdt_sync` and `crdt_update` are typed JSON
   messages with base64 payloads; merged updates broadcast to all subscribers.

## Alternatives considered

| Alternative | Why not now |
| ----------- | ----------- |
| Whole-page `If-Match` only | Simple, but concurrent text edits are lost or rejected. |
| Automerge | Good CRDT, but Yrs has a mature text/state-vector model and compact updates. |
| Custom CRDT | Too much correctness risk; use a proven implementation. |
| CRDT for block structure too | Much larger model change; text is the immediate collaboration need. |
| Persist every CRDT update | Write amplification; the projected snapshot is enough for this slice. |
| Client-only CRDT | Server needs to merge and project for REST clients and durability. |

## Consequences

Positive:

- Concurrent text edits converge without page-level conflicts.
- Updates are compact and resume via state vectors.
- REST clients still see merged text through the persisted snapshot.
- The change log records one update per applied CRDT batch.

Costs:

- CRDT history/undo is not durable; a restart keeps the merged text but not the
  operation history.
- Rich-text annotations are not CRDT-merged; a single plain-text object is
  rebuilt when the block has at most one segment.
- The egui editor does not use Yrs yet; a client adapter is the next step.
- A REST write resets CRDT history for that page.

## Follow-up

- Add a client-side Yrs adapter and awareness/cursor payloads.
- Persist CRDT state/undo if history must survive restarts.
- Add CRDT for block structure or properties only if the domain requires it.
- Add multi-node update fan-out when the server is horizontally scaled.
