# spec-081 - Refactoring roadmap and implementation notes

- Status: implemented (2026-11-12)
- Related: `spec-078`, `spec-079`, `spec-080`
- Review sources: Claude and ChatGPT general-refactoring reviews

## Purpose and scope

The general-refactoring reviews proposed a broad set of structural changes.
This spec records the changes implemented in this pass and gives the remaining
items a concrete, risk-ordered roadmap.

Scope:

- `docs/refactoring-roadmap.md`;
- updates to `docs/index.md`, `AGENTS.md` and `README.md`;
- a status note in the relevant specs.

Out of scope:

- implementing the deferred server/client/UI refactors in this pass;
- a new crate split or a framework.

## Acceptance criteria

P1. **Given** a contributor reading the refactoring review, **when** they look
up an item, **then** the roadmap says whether it is implemented or deferred
and what the first slice is.

P2. **Given** the implemented route table, CRDT helpers and sidebar
decomposition, **when** a contributor changes one of them, **then** the roadmap
and the relevant spec explain the design.

P3. **Given** a deferred item, **when** it is picked up, **then** the roadmap
states the behavior-preserving test gate and the one-family-per-change rule.

## Test scenarios

- Documentation link, Mermaid and table checks pass.
- Full quality gate passes.
- The route-policy, CRDT and sidebar tests remain green.

## Design

- One roadmap document, grouped by layer (server, client/UI, domain/infrastructure).
- Each deferred item has a "why deferred" and a "suggested first slice" column.
- The roadmap is linked from the docs index, AGENTS and README.

## Plan / tasks

- [x] Write `docs/refactoring-roadmap.md`
- [x] Add the docs index and README entries
- [x] Add the AGENTS maintenance rule
- [x] Run the full gate and commit
