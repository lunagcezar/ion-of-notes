# spec-010 — Improved architecture guidelines and composition root

## Purpose and scope

The person added an architecture review for Rust desktop applications, the
[Rust API Guidelines](https://github.com/rust-lang/api-guidelines), and
[The Rust Programming Language, ch. 7](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html),
with the standing instruction: **always add improved project guidelines in
`AGENTS.md`**. This spec covers, and only covers:

- Adding the improved guidelines to `AGENTS.md`: pragmatic Modular Monolith +
  Clean Architecture + selective DDD; two distinct decisions (code
  modularization vs business architecture); layer responsibilities and
  dependency direction; the `app`/bootstrap layer; Rust Book ch. 7 visibility
  rules (private by default, `pub(crate)`, `pub use` facade); file-creation
  heuristics; pragmatic DDD invariants and context errors; desktop
  presentation as an input adapter; the feature-based growth path; team
  conventions; and the new external references.
- A single contained refactor that applies the review: a new `src/app.rs`
  **composition root** that wires concrete adapters to ports, so `main.rs`
  stays CLI wiring only and the wiring is written once (DRY / SRP / DIP).
  `data_path()` moves from `ui/palette.rs` to `store.rs` (infrastructure).
- Recording the decision in `docs/adr-002-architecture-guidelines.md` and the
  change in `CHANGELOG.md`.

Out of scope: adopting new dependencies, changing the API contract or any
behavior, splitting into multiple crates, creating empty feature/layer
folders, adding accounts/collaboration, or any visual redesign. The 29
reference pictures remain the visual source of truth.

## Evidence used

- The 29 pictures in `reference/` were re-opened: dark tab strip, sidebar,
  breadcrumb bar, centered page column, and the Library / Share / Inbox /
  Settings opened states. This spec is structural only, so the window must be
  unchanged after it.
- The architecture review, the Rust API Guidelines, and Rust Book ch. 7.

## Acceptance criteria (Given / When / Then)

1. **Given** `AGENTS.md`, **when** inspected, **then** it contains the improved
   guidelines (layers, dependency direction, visibility rules, file-creation
   heuristics, DDD invariants, desktop adapter flow, feature-based growth
   path, team conventions) and the two new external references.
2. **Given** `src/`, **when** inspected, **then** there is an `app` composition
   root exposing `build_server` / `build_desktop`, and neither `main.rs` nor
   `ui/app.rs` assembles the `WorkspaceServer`/`Client` graph itself.
3. **Given** `src/ui/palette.rs`, **when** inspected, **then** `data_path()` is
   gone (it belongs to `store.rs`, the persistence adapter).
4. **Given** `cargo build` and `cargo test`, **when** run, **then** both are
   clean and green, including the new `tests/app.rs` composition-root tests.
5. **Given** `cargo run -- --screenshot shots/now.png`, **when** the fresh PNG
   is compared with the 29 pictures in `reference/`, **then** the dark
   sidebar/breadcrumb/page layout is unchanged and the image is not blank.

## Test scenarios (written before implementation)

- `build_desktop_at_seeds_and_serves_the_workspace` — composing at a temp path
  seeds the demo workspace and the client serves that same snapshot.
- `build_server_at_answers_health` — a composed server answers `GET /health`
  with `200`.
- Both use the shared `tests/common/mod.rs` temp helpers (DRY); the red step
  is the unresolved `not_notion::app` module observed before `src/app.rs`
  exists.

## Plan

1. Red: write `tests/app.rs`, run `cargo test --test app`, observe the missing
   `app` module.
2. Green: add `src/app.rs`; re-export from `lib.rs`; move `data_path()` to
   `store.rs`; remove it from `ui/palette.rs` and the `ui.rs` re-export;
   delegate composition from `main.rs` and `ui/app.rs`.
3. `cargo build`, `cargo test`, clean any warnings/errors.
4. Update `AGENTS.md`, add `docs/adr-002`, add the `CHANGELOG.md` entry.
5. Fresh `--screenshot` and compare with `reference/`.

## Notes

- This is a refactor: no production behavior changes, so the existing 57 tests
  plus the two new ones are the regression suite (as in spec-008). No
  `#[cfg(test)]` blocks are added to `src/`.
- The review is adopted as *guidelines*, not a literal file tree: no empty
  `domain/application/infrastructure` folders, no premature abstractions, and
  no microservices for a single-user local desktop app.
