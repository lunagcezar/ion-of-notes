# spec-125 - Deterministic workspace invariant fuzzing

- Status: implemented (2026-11-24)
- Related: `spec-022`, `spec-103`, `spec-109`
- Review source: deferred property/fuzz test item in `docs/review-response.md`

## Purpose and scope

The workspace keeps derived state (page index, block ownership, search cache)
beside the serialized shape. Unit tests cover each operation, but not long
mixed sequences of adds, removes, block edits and version bumps.

This spec adds a deterministic fuzz-style test:

- a fixed LCG drives 400 mixed mutations over a seeded workspace;
- after every step it checks:
  - page ids remain unique;
  - `page(id)` finds every page;
  - `children(parent)` equals the parent filter;
  - every block maps back to its owning page;
  - breadcrumbs are non-empty;
  - search only returns existing page ids;
  - the block count is at least the page count.

No new dependency and no wall-clock/random-seed nondeterminism: the same seed
always produces the same sequence.

Out of scope:

- coverage-guided fuzzing (cargo-fuzz);
- persistence round-trips inside the fuzz loop (covered by store tests);
- UI state machines.

## Acceptance criteria

P1. **Given** the fixed seed, **when** the test runs repeatedly, **then** it
produces the same sequence and passes.

P2. **Given** a page/block mutation, **then** block ownership and page lookup
remain consistent.

P3. **Given** a removal, **then** descendants and search documents disappear
from lookups.

## Test scenarios

- `tests/model.rs`
  (`randomized_workspace_mutations_keep_indices_and_block_ownership_consistent`).
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Mixed mutation loop | `tests/model.rs` |
| LCG | inline, fixed seed |

## Plan / tasks

- [x] Add the deterministic LCG and mixed operations.
- [x] Assert page/child/block/search invariants after every step.
- [x] Keep the test fast and dependency-free.
