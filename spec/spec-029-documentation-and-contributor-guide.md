# spec-029 — Documentation and contributor guide

## Purpose and scope

The repository has 29 specs, five ADRs, requirements, a data model, a visual
inventory, an observation inventory and a reverse-engineering log. A review
found that several documents disagreed with the code, that the requirements
had non-functional items in a functional section, and that there was no single
entry point for contributors or agents.

This spec covers, and only covers:

- a documentation hub (`docs/index.md`) with reading order and
  source-of-truth rules;
- a didactic architecture guide (`docs/architecture.md`) with Mermaid
  diagrams for context, layers, request flow, persistence and collaboration;
- a contributor guide (`docs/contributor-guide.md`) for the SDD/TDD loop;
- a testing guide (`docs/testing.md`) with the test-crate map and fakes;
- a glossary (`docs/glossary.md`) of the project vocabulary;
- a review-findings document (`docs/review-findings.md`) recording what the
  review found and what remains open;
- an API contract reference (`docs/api-contract.md`) separating the
  implemented loopback API from the decided `/api/v1` target;
- a clean-room guide (`docs/clean-room.md`) with allowed sources, evidence
  levels, stop conditions and the B1–B8 status;
- a spec index (`spec/README.md`);
- corrections to every stale document found during the review.

Out of scope:

- any behavior, route, DTO, persistence or visual change;
- new dependencies;
- implementing A9, B4/B5 or the production server;
- rewriting retired or historical specs as if they had never happened.

## Evidence

- A full read of every Markdown file in the repository on 2026-11-08.
- Cross-checks against `src/`, `tests/`, `Cargo.toml`, `locales/` and
  `.github/workflows/ci.yml`.
- Visual evidence: **98** pictures in the workspace `reference/` folder
  (original 29 + later build) and **72** local `screenshots/` pictures; the
  inventory was expanded to cover both sets.
- `cargo test --all-targets`: 115 tests, all green.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`: clean.
- Findings recorded in `docs/review-findings.md`.

## Acceptance criteria

1. **Given** a new contributor, **when** they open `docs/index.md`, **then**
   they see a reading order, a document catalog and the source-of-truth rules.
2. **Given** an agent starting a task, **when** they read `AGENTS.md`, **then**
   the module tree, API table and docs-maintenance list match the current code.
3. **Given** `docs/requirements.md`, **when** it is inspected, **then** all
   non-functional requirements are in §2 and each requirement status matches
   the implementing spec.
4. **Given** the five ADRs, **when** they are read, **then** each states its
   current phase and does not describe implemented work as future work.
5. **Given** `docs/api-contract.md`, **when** it is compared with
   `src/server.rs`, **then** every implemented route, DTO, status code and
   query parameter is documented, and the decided `/api/v1` target is clearly
   separated.
6. **Given** `docs/clean-room.md`, **when** it is read, **then** allowed and
   forbidden sources, evidence levels, stop conditions and the B1–B8 status are
   explicit.
7. **Given** `spec/README.md`, **when** it is read, **then** every spec has a
   status and the open work is listed.
8. **Given** `docs/testing.md`, **when** it is read, **then** the test-crate
   map matches `tests/` and the red/green rules match `AGENTS.md`.
9. **Given** the repository, **when** `cargo test --all-targets`,
   `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` run,
   **then** all are clean.
10. **Given** a Mermaid block or relative link, **when** it is checked,
    **then** it is syntactically valid and points to an existing file.
11. **Given** the visual evidence, **when** the docs refer to it, **then** they
    state the current counts (98 in `reference/`, 72 in `screenshots/`) and do
    not imply that only 29 pictures exist.

## Test scenarios (written before implementation)

This is a documentation-only spec; the "red" step is the review itself (a list
of broken claims) and the verification is structural plus the existing suite.

- **Documentation checks (manual/scripted):**
  - every relative Markdown link resolves to an existing file;
  - every Mermaid block has a recognized diagram type and balanced fences;
  - no document still says "65 tests", "seven modules", "CollabStore is next",
    or "FR-01..FR-71";
  - every spec in `spec/README.md` exists, and every spec file is listed.
- **Existing suite (regression):**
  - `cargo test --all-targets` — 115 tests green;
  - `cargo clippy --all-targets -- -D warnings` — clean;
  - `cargo fmt --check` — clean.
- **Visual regression:** no UI change, so no new screenshot is required; the
  default window is unchanged.

## Design

### New documents

| File | Responsibility |
| ---- | -------------- |
| `docs/index.md` | Entry point, reading order, document catalog, source-of-truth rules |
| `docs/architecture.md` | System context, layers, module map, request/persistence flows, design principles |
| `docs/contributor-guide.md` | SDD/TDD workflow, where code belongs, common pitfalls, commit checklist |
| `docs/testing.md` | Test strategy, test-crate map, fakes, determinism, visual checks |
| `docs/glossary.md` | Shared vocabulary by concern |
| `docs/review-findings.md` | Findings, severity, resolution and follow-up queue |
| `docs/api-contract.md` | Implemented routes/DTOs vs decided `/api/v1`/WS contract |
| `docs/clean-room.md` | Allowed/forbidden sources, evidence levels, B1–B8, stop conditions |
| `spec/README.md` | Spec index, lifecycle, status vocabulary, open work |

### Updated documents

| File | Change |
| ---- | ------ |
| `README.md` | Current overview, docs hub link, status, module list, clean-room note |
| `AGENTS.md` | Current module tree, API table, docs-maintenance list, doc map |
| `CHANGELOG.md` | This spec's entry |
| `docs/requirements.md` | NFR-10..15 moved to §2; stale statuses fixed; NFR-16 added |
| `docs/data-model.md` | `Workspace.settings`, ER diagram, version/comment/export notes |
| `docs/unknowns.md` | 2026-11-08 status, U-06/U-08 updated, stale test count removed |
| `docs/api-observations.md` | FR range, implementation status, change log |
| `docs/reverse-engineering-log.md` | 2026-11-08 review entry, B-phase status |
| `docs/visual-reference-inventory.md` | Implemented vs remaining gaps |
| `docs/adr-001` | Current v1 status note |
| `docs/adr-002` | Link to the architecture guide; later split note |
| `docs/adr-003` | Implementation status; `CollabStore` and export ownership corrected |
| `docs/adr-004` | Current module count; later refactors |
| `docs/adr-005` | B1/B2 done, B3 blocked; B4 reviewed later on 2026-11-08 |
| `spec-002` | Historical note: tests now in `tests/model.rs` |
| `spec-008` | Current test map pointer |
| `spec-011` | `CollabStore` closed by spec-019 |
| `spec-014` | Plan checkboxes completed with notes |
| `spec-016` | Plan checkboxes and B-phase status corrected |

### Why this shape

- **One hub, many focused documents.** A single long file would be hard to
  navigate; many tiny files would fragment the reading order. The hub links to
  cohesive documents by audience.
- **Diagrams plus prose.** Mermaid helps a contributor see the flow; the prose
  stands alone if a renderer is unavailable.
- **Status notes on historical documents.** Rewriting history hides why a
  decision changed. A dated note preserves the audit trail.
- **One source of truth per concern.** Requirements, data model, API contract,
  architecture and clean-room evidence each have one owner document; the rest
  link to them.

## Plan / tasks

- [x] Inventory every Markdown file and cross-check it with the code.
- [x] Record findings with severity and evidence.
- [x] Create `docs/index.md`, `docs/architecture.md`,
  `docs/contributor-guide.md`.
- [x] Create `docs/testing.md`, `docs/glossary.md`, `docs/review-findings.md`,
  `docs/api-contract.md`, `docs/clean-room.md`.
- [x] Create `spec/README.md` and this spec.
- [x] Update `README.md`, `AGENTS.md`, `CHANGELOG.md`,
  `docs/requirements.md`, `docs/data-model.md`, `docs/unknowns.md`,
  `docs/api-observations.md`, `docs/reverse-engineering-log.md`,
  `docs/visual-reference-inventory.md`.
- [x] Add status notes to ADR-001..005 and specs 002, 008, 011, 014, 016.
- [x] Verify relative links, Mermaid blocks and stale phrases.
- [x] Run tests, clippy and fmt; confirm 115 tests green.
- [ ] Commit with a message naming spec-029.

## Requirement mapping

| Requirement | How this spec addresses it |
| ----------- | -------------------------- |
| R-02 (English documentation) | All new documents are in English |
| R-05 / NFR-02 (SDD/TDD) | The contributor and testing guides document the loop |
| NFR-07 (clean room) | `docs/clean-room.md` restates the boundary and stop conditions |
| NFR-08 (tests in `tests/`) | `docs/testing.md` documents the layout and rules |
| NFR-09 (ports/responsibility) | `docs/architecture.md` documents the seams and layers |
| NFR-16 (documentation upkeep, new) | `docs/index.md` and `AGENTS.md` define ownership and update rules |
| FR-68 (i18n) | Status corrected to implemented (spec-028) |
| FR-71 (settings row filtering) | Status kept open, tied to spec-016 A9 |

## Notes

- No production code changed. The only code-adjacent change is documentation;
  the existing 115-test suite is the regression guard.
- The review deliberately did **not** start B4/B5 at the time: the person would
  provide the Notion public API links first. **Update:** the links were provided
  and B4 was reviewed on 2026-11-08 (`docs/public-api-analysis.md`, ADR-006,
  specs 030-037); spec-030 is implemented; B5 follows the next v2 slices.
