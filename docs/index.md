# Documentation index — ion-of-notes

This is the entry point for contributors and agents. It explains what to read,
in what order, and which document is the source of truth for each concern.

## Start here

| You are… | Read in this order |
| -------- | ------------------ |
| New contributor | `README.md` → `docs/architecture.md` → `docs/contributor-guide.md` → `docs/testing.md` → `docs/glossary.md` |
| Agent starting a task | `AGENTS.md` → `spec/README.md` → `docs/architecture.md` → the relevant spec → `docs/review-findings.md` |
| Working on the server/API | `docs/api-contract.md` → `docs/public-api-analysis.md` → `docs/api-observations.md` → `docs/clean-room.md` → ADR-003/ADR-006/ADR-007/ADR-008/ADR-009/ADR-010/ADR-011/ADR-012/ADR-013/ADR-014 |
| Reviewing quality | `docs/review-findings.md` → `docs/testing.md` → `CHANGELOG.md` |
| Working on visuals | `docs/visual-reference-inventory.md` → `reference/` and `screenshots/` → the relevant spec |

## Current status (2026-11-08)

- **Specs:** `spec-001` … `spec-045` implemented; `spec-007` retired;
  `spec-016` is a plan with B5 done; `spec-029` is the documentation
  overhaul; `spec-030` is the v2 foundation; `spec-031` adds pagination;
  `spec-032` adds rich text/blocks; `spec-033` adds properties/data sources;
  `spec-034` adds views; `spec-035` adds comments/users/files; `spec-036` adds
  webhooks; `spec-037` adds limits and operations; `spec-038` adds the Axum
  web server and B5 contract suite; `spec-039` adds sessions, Argon2id and
  SQLite storage; `spec-040` adds the WebSocket change feed and presence;
  `spec-041` adds TLS webhooks and shared SQLite rate limits; `spec-042` adds
  Yrs CRDT text merge; `spec-043` adds the client session and awareness
  cursors; `spec-044` adds the remote HTTP/WebSocket transport; `spec-045` adds
  the server login/settings UI.
- **Tests:** 252 integration tests across `tests/`; `cargo clippy --all-targets
  -- -D warnings` and `cargo fmt --check` are clean.
- **Runtime:** in-process server + Axum HTTP/WebSocket adapter; `--database`
  selects SQLite with Argon2id accounts and sessions. The desktop can connect
  remotely with `--server`/`--token` or Settings → Server. PostgreSQL remains
  next.
- **Collaboration:** members, shares, sync log and version snapshots persist in
  `collab.json` or SQLite. `GET /ws/workspace/{id}` streams changes with
  presence and resume; Yrs CRDT merges page title/block text server-side and the
  desktop editor uses a client Yrs session with awareness cursors.
- **Clean room:** strict public-behavior-only observation. No capture, proxy,
  Wireshark, source maps or decompilation. B3 is blocked; **B4 public docs were
  reviewed on 2026-11-08** (`docs/public-api-analysis.md`); the v2 foundation,
  pagination, rich text/blocks, properties/data sources, views,
  comments/users/files, webhooks, limits and the Axum adapter are implemented
  (specs 030–038); sessions/SQLite (spec-039), the WebSocket change feed
  (spec-040), TLS webhooks/shared limits (spec-041), Yrs CRDT merge (spec-042),
  client awareness (spec-043), the remote transport (spec-044) and the server
  login UI (spec-045) are implemented. B5 is done.
- **Localization:** core navigation, Share, Library, page actions, comments and
  Settings are translated into `en`, `pt-BR` and `eo`; remaining strings are
  migrated incrementally.
- **Visual evidence:** 98 pictures in the workspace `reference/` folder
  (original 29 + later build) and 72 local `screenshots/` pictures; see
  `docs/visual-reference-inventory.md`.

## Documentation map

```mermaid
flowchart TD
    README[README.md] --> INDEX[docs/index.md]
    INDEX --> ARCH[docs/architecture.md]
    INDEX --> GUIDE[docs/contributor-guide.md]
    INDEX --> TEST[docs/testing.md]
    INDEX --> GLOSS[docs/glossary.md]
    INDEX --> REVIEW[docs/review-findings.md]
    INDEX --> API[docs/api-contract.md]
    INDEX --> PUBLIC[docs/public-api-analysis.md]
    API --> PUBLIC
    INDEX --> CLEAN[docs/clean-room.md]
    INDEX --> REQ[docs/requirements.md]
    INDEX --> DATA[docs/data-model.md]
    INDEX --> UNK[docs/unknowns.md]
    INDEX --> VIS[docs/visual-reference-inventory.md]
    INDEX --> ADR[docs/adr-*.md]
    INDEX --> SPECS[spec/README.md]
    AGENTS[AGENTS.md] --> INDEX
    SPECS --> ARCH
    REVIEW --> SPECS
```

## Document catalog

| Document | Audience | What it is the source of truth for |
| -------- | -------- | ---------------------------------- |
| `AGENTS.md` | agents, contributors | Working rules, architecture guidelines, module tree, commands, conventions, commit rules |
| `README.md` | everyone | Project overview, quick start, feature/status summary |
| `docs/index.md` | everyone | Reading order and documentation map |
| `docs/architecture.md` | contributors, agents | System context, layers, modules, request/persistence flows, diagrams |
| `docs/contributor-guide.md` | contributors, agents | Step-by-step SDD/TDD workflow and how to add features |
| `docs/testing.md` | contributors, agents | Test strategy, test crates, fakes, red/green discipline |
| `docs/glossary.md` | everyone | Shared vocabulary (domain, ports, adapters, DTOs, CRDT, clean room, …) |
| `docs/review-findings.md` | maintainers, reviewers | Findings from the architecture/performance review and their resolution |
| `docs/requirements.md` | everyone | Functional and non-functional requirements, requirement → spec map |
| `docs/data-model.md` | contributors, agents | Entities, fields, relationships, invariants, persistence shape |
| `docs/api-contract.md` | client/server contributors | Current loopback API, decided `/api/v1` target, DTOs, status codes |
| `docs/api-observations.md` | clean-room researchers | UI/public evidence inventory and unknown server behavior |
| `docs/public-api-analysis.md` | architects, API contributors | B4 public Notion API analysis, comparison and roadmap |
| `docs/clean-room.md` | everyone | Allowed sources, observation levels, stop conditions, Wireshark guidance |
| `docs/reverse-engineering-log.md` | reviewers, agents | Auditable clean-room log of observations vs inferences |
| `docs/visual-reference-inventory.md` | UI contributors | What the 72 `screenshots/` pictures show and their gaps |
| `docs/unknowns.md` | everyone | Open reconstruction questions and their status |
| `docs/adr-001…006` | maintainers | Accepted architecture, API, modularization, clean-room and public-API-alignment decisions |
| `scripts/README.md` | contributors, agents | Reusable link/Mermaid/table/test/quality checks |
| `spec/README.md` | everyone | Spec index, status, and the SDD lifecycle |
| `CHANGELOG.md` | everyone | What changed, in what spec, with verification notes |

## Source-of-truth rules

1. **Behavior requirements** live in `docs/requirements.md`; a spec records the
   slice that implements them.
2. **Data shape** lives in `docs/data-model.md`; Rust structs are the executable
   form.
3. **Architecture decisions** live in ADRs; `AGENTS.md` summarizes the standing
   guidelines.
4. **The current API** is `docs/api-contract.md`; the original service's
   contract is unknown and must not be inferred from our implementation.
5. **Visual evidence** is `reference/` (98 pictures: the original 29 plus the
   later build) plus `screenshots/` (72 local later-build pictures);
   `docs/visual-reference-inventory.md` summarizes them.
6. **Clean-room evidence** is `docs/api-observations.md` and
   `docs/reverse-engineering-log.md`; no capture is authorized under B1.
7. **Public API evidence** is `docs/public-api-analysis.md` plus the
   **Observed (public docs)** rows in `docs/api-observations.md`. It describes
   the public integration contract, not the original private server.

## Keeping docs current

- Behavior change → update requirements + the relevant spec.
- New entity/field → update data model.
- New route/DTO/status → update `docs/api-contract.md`.
- New module or dependency direction change → update `AGENTS.md` and
  `docs/architecture.md`.
- New visual surface → update `docs/visual-reference-inventory.md` if it comes
  from the pictures.
- Completed spec → update `CHANGELOG.md`, `spec/README.md`, and commit.
