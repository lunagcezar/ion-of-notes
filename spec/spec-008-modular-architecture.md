# spec-008 — Modular architecture, separated tests, stack evaluation

## Purpose and scope

The person observed that **tests and implementation were living in the same
file** and asked for a more modular, reusable project, asking to search for
good architectural patterns for Rust and to apply DRY and SOLID always. In the
same message a suggested technology list was provided for evaluation ("Do you
agree with its choices?").

This spec covers, and only covers:

- Moving every `#[cfg(test)] mod tests` block out of `src/` into one focused
  integration-test crate per module under `tests/`, with shared helpers in
  `tests/common/mod.rs` (the layout recommended by the Rust Book, ch. 11
  "Test Organization", and ch. 12 "Refactoring to Improve Modularity").
- Removing the duplicated temporary-file helpers (DRY) and the duplicated
  `load_or_seed_creates_file` test.
- Splitting the 1700-line `ui.rs` god module into single-responsibility
  submodules: `ui/palette.rs`, `ui/icons.rs`, `ui/app.rs`, `ui/render.rs`,
  `ui/dialogs.rs`, with `ui.rs` reduced to declarations and re-exports (SRP).
- Introducing a `WorkspaceStore` persistence **port** with `JsonStore` and
  `NoStore` adapters. `WorkspaceServer` depends on the abstraction, not on a
  concrete file (DIP / ports-and-adapters); new backends can be added without
  touching domain logic (OCP).
- Recording an architecture decision record for the suggested library list:
  `docs/adr-001-stack-evaluation.md`.
- Keeping the public API, the dark visual design, and all 29 reference
  pictures matched. No visual redesign, no new runtime dependency.

Out of scope: adopting any of the suggested libraries, changing behavior,
changing the API contract, or adding accounts/collaboration.

## Research summary (patterns applied)

- Rust Book ch. 12: `main.rs` only wires the program; logic lives in `lib.rs`;
  each function has one responsibility; group configuration into structs.
- Rust Book ch. 11: integration tests live in `tests/`, use only the public
  API, and shared helpers belong in `tests/common/mod.rs` (a `tests/common.rs`
  would create a bogus empty test target).
- Rust API Guidelines: prefer traits and generic seams for flexibility and
  future-proofing; keep public interfaces small and documented.
- Hexagonal / ports-and-adapters: the core declares ports (`WorkspaceApi`,
  `WorkspaceStore`); adapters (JSON file, loopback HTTP) implement them.
- SOLID: one reason to change per module (S), open for new adapters without
  editing the core (O), clients depend on traits (D), small focused traits (I).
- DRY: one shared test helper module instead of four copies of `temp_path`.

## Acceptance criteria (Given / When / Then)

1. **Given** any file under `src/`, **when** it is inspected, **then** it
   contains no `#[cfg(test)] mod tests` block and no test function.
2. **Given** the `tests/` directory, **when** enumerated, **then** there is one
   test file per module and one `tests/common/mod.rs` holding every shared
   helper; no helper is duplicated in two test files.
3. **Given** `src/ui.rs`, **when** inspected, **then** it is a small module root
   that declares and re-exports the five `ui/` submodules, and each submodule
   has a single responsibility.
4. **Given** a custom `WorkspaceStore` implementation, **when** it is injected
   with `WorkspaceServer::with_store`, **then** a mutation persists through that
   store and no file is written; the in-process, `with_path`, and `--serve`
   constructors keep their previous behavior.
5. **Given** `cargo build` and `cargo test`, **when** run, **then** both are
   green with no errors and the test count is unchanged (plus the new port
   tests).
6. **Given** `cargo run -- --screenshot shots/now.png`, **when** the fresh PNG
   is compared with the 29 pictures in `reference/`, **then** the dark
   sidebar/breadcrumb/page layout is unchanged and the image is not blank.

## Test scenarios (written before implementation)

- All existing 48 tests are preserved verbatim in behavior, relocated to
  `tests/model.rs`, `tests/store.rs`, `tests/server.rs`, `tests/client.rs`,
  `tests/cli.rs`, `tests/edition.rs`, `tests/screenshot.rs`, `tests/ui.rs`,
  and `tests/integration.rs`.
  > Note: `tests/edition.rs` was later removed together with the retired
  > `spec-007`; `tests/collab.rs`, `tests/app.rs` and `tests/fidelity.rs` were
  > added in later sessions.
- `server_accepts_a_swappable_store_without_touching_disk` — a `RecordingStore`
  injected with `with_store` receives exactly one snapshot per mutation.
- `json_store_persists_to_its_configured_path` — `JsonStore::new(path)`
  writes a workspace that `store::load` can read back.
- `no_store_persist_is_a_noop` — `NoStore` succeeds without I/O.
- `with_path_still_persists_after_a_mutation` — the original constructor keeps
  its on-disk behavior (regression guard for the DIP refactor).
- The duplicate `load_or_seed_creates_file` scenario exists only once, in
  `tests/store.rs`.

## Plan

1. Write this spec and `docs/adr-001-stack-evaluation.md` first.
2. Red: write all relocated tests and the new `WorkspaceStore` tests, then run
   `cargo test` and observe the expected compile failure (`with_store` and the
   port do not exist yet).
3. Green: add the `WorkspaceStore` port/adapters to `store.rs`, make
   `WorkspaceServer` depend on it, and re-export from `lib.rs`.
4. Green: split `ui.rs` into the five submodules, preserving behavior.
5. `cargo build`, `cargo test`, then `--screenshot` and compare with
   `reference/`.
6. Update `AGENTS.md`, `docs/requirements.md`, `README.md`, and
   `CHANGELOG.md`.

## Notes

- This is a refactor: no production behavior changes, so the existing tests
  are the regression suite. The red step is produced by the new port tests.
- The evaluation of the suggested libraries is a decision record, not an
  implementation. See `docs/adr-001-stack-evaluation.md`.
- **Historical note (2026-11-08):** the test list above is the layout at the
  time of this spec. The suite now has 17 integration-test crates (115 tests);
  `docs/testing.md` owns the current map. The UI split into five modules here
  was extended by `spec-017` (UI) and `spec-018` (domain/infrastructure).
