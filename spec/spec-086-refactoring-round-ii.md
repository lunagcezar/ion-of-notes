# spec-086 - Refactoring round II

- Status: implemented (2026-11-12)
- Related: `spec-078` .. `spec-085`
- Review sources: Claude and ChatGPT general-refactoring reviews

## Purpose and scope

This spec records the second general-refactoring round and the remaining
risk-ordered work. The implemented slices are the request pipeline, transport
segregation, resource-oriented client and transport adapter split.

Scope:

- `spec-082` request pipeline stages;
- `spec-083` `RequestTransport` / `CollabTransport`;
- `spec-084` client resource modules;
- `spec-085` `server/web` and `remote` adapter split;
- `docs/refactoring-roadmap.md` updated with implemented/deferred status.

Out of scope:

- typed handler errors and service extraction;
- domain events;
- UI state/view/command grouping;
- DTO separation;
- normalized SQLite and benchmarks.

## Acceptance criteria

P1. **Given** the refactoring roadmap, **when** a contributor looks up an item,
**then** it says whether it is implemented or deferred and gives a first slice.

P2. **Given** the pipeline, client and transport changes, **when** the full gate
runs, **then** behavior is unchanged and all tests pass.

P3. **Given** the deferred UI/domain/database items, **when** they are picked
up, **then** the roadmap states the risk and the measurement prerequisite.

## Test scenarios

- Full `scripts/quality.ps1` gate.
- Documentation link/Mermaid/table checks.
- Existing client, remote, web, smoke, CRDT and UI tests.

## Design

- One roadmap document remains the status source.
- Each deferred item names why it is deferred and the first safe slice.
- No new abstraction is introduced for the deferred items.

## Plan / tasks

- [x] Implement pipeline, transport segregation, client modules and adapter split
- [x] Update the roadmap and docs index
- [x] Run the full gate
- [x] Commit
