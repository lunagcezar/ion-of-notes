# ADR-002 — Architecture guidelines for the Rust desktop app

**Status:** Accepted (guidelines + one contained refactor; no behavior change)
**Date:** 2026-11-05
**Context:** The person added an architecture review (ChatGPT) plus the
[Rust API Guidelines](https://github.com/rust-lang/api-guidelines) and
[The Rust Programming Language, ch. 7](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html),
and asked that improved project guidelines always be added to `AGENTS.md`.
The review discusses modular monoliths, Clean Architecture, pragmatic DDD,
feature-based module organization, visibility rules, and the `app`/bootstrap
layer. The app is a single-crate, clean-room, local-first desktop
reconstruction whose dark layout already matched the 29 pictures in
`reference/` at the time (the set has since grown to 98).

## Decision

We adopt the review as **guidelines, not a literal blueprint**, and record it
in `AGENTS.md` under "Improved architecture guidelines". Concretely:

- Keep a **pragmatic Modular Monolith**: one executable/crate, Clean
  Architecture through the `WorkspaceApi` / `WorkspaceStore` ports, and
  **selective** DDD. Do not add microservices, premature abstractions, or a
  class/struct for every noun.
- Treat **code modularization** and **business architecture** as two separate
  decisions. DRY and SOLID are tools: DRY avoids duplicating *rules*, not any
  similar-looking code; SOLID is applied mainly as SRP and DIP.
- Enforce the **dependency direction**: the domain (`model`) knows nothing of
  infrastructure or GUI; adapters implement ports; `presentation` is an input
  adapter that delegates to use cases and holds no business rules.
- Apply **Rust Book ch. 7 visibility discipline**: private by default, `pub`
  only at real boundaries, `pub(crate)` for crate-internal sharing, `pub use`
  for a clean public facade, modules as encapsulation boundaries.
- Use **one file per coherent unit of responsibility** (no one-struct-per-file
  fragmentation, no magic line count), and **context-specific error types**
  with validated constructors when the domain grows real invariants. Our
  pragmatic tradeoff: serde-friendly public-field model structs, with
  invariants enforced at the `server` / `Workspace` boundaries today.
- Prefer **feature-based modules** (`src/modules/<feature>/{domain,
  application, infrastructure}`) as the *growth path* for large projects, but
  keep the current layered single-crate layout while there is one bounded
  context. Migrate one feature at a time; do not create empty scaffolds now.
  Architecture is not measured by file count.
- Add the one structural piece the review calls for and the code lacked: an
  **`app` composition root (bootstrap)** that connects concrete adapters to
  ports. `main.rs` remains CLI wiring only, and neither `main.rs` nor the
  window assembles the object graph itself.

## Change made (spec-010)

- New `src/app.rs`: `build_server()` / `build_server_at(path)` and
  `build_desktop()` / `build_desktop_at(path)` compose
  `Workspace` -> `WorkspaceServer` (with the `JsonStore` adapter) -> `Client`.
- `data_path()` moved from `ui/palette.rs` (a theming module) to `store.rs`
  (the persistence adapter), since the file location is an infrastructure
  decision, not a visual one. `ui.rs` no longer re-exports it.
- `main.rs` (`--serve`) and `ui/app.rs` (desktop) now request the ready-made
  graph from `app` instead of duplicating the wiring (DRY / SRP).
- `tests/app.rs`: two integration tests for the composition root, using the
  shared `tests/common/mod.rs` temp-path helpers (TDD red -> green).
- `AGENTS.md`: improved guidelines, layer table, dependency direction,
  visibility rules, file-creation heuristics, DDD invariants, desktop adapter
  flow, feature-based growth path, team conventions, and the new references.

## Later application (2026-11-08)

The guidelines were applied again without changing this decision:

- `spec-017` split the three large UI files by responsibility
  (`ui/commands`, `view`, `input`, `apply`, `render/*`, `dialogs/*`).
- `spec-018` split `model`, `collab` and `server` (`model/*`,
  `collab/export.rs`, `server/http.rs`).
- `spec-019` added the `CollabStore` port and `JsonCollabStore` adapter.
- `spec-020` added the pure `PageDiff` / `CommitPlan` / `PendingEdits` path.
- `spec-022` added the derived `WorkspaceIndex` and version retention.

The didactic companion is `docs/architecture.md`; the current module tree is in
`AGENTS.md` and `docs/architecture.md` §3. No `src/modules/` scaffold and no new
crate were created, as decided here.

## Consequences

- Positive: the wiring lives once and is testable without a GUI; the
  presentation layer no longer owns an infrastructure detail; the guidelines
  are explicit for future sessions.
- Neutral: this is a pure refactor — no production behavior, API contract, or
  visual change. The existing test suite is the regression guard; a fresh
  `--screenshot` is compared with `reference/` (98 pictures today; 29 at the
  time of this ADR).
- Deferred: extracting features into `src/modules/` or separate crates, and
  private-field domain value objects, until there is a real second bounded
  context or real domain behavior.
