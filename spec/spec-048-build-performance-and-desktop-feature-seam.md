# spec-048 - Build performance and the desktop feature seam

- Status: implemented (2026-11-10)
- Related: `spec-046`, `spec-047`, `docs/adr-004-modularization-decision.md`,
  `docs/testing.md`, `docs/review-findings.md` section 9

## Purpose and scope

`cargo test --all-targets` builds 33 integration-test executables. Each one
links the whole application, including `eframe`/`egui`/`wgpu`, even though only
four test crates use the desktop UI. With full debug info each test binary
writes a 220-270 MB PDB, so the parallel link phase exhausts a 16 GB machine
and pages heavily. This spec makes the test build lighter **without** turning
off incremental compilation and **without** capping Cargo's job count.

In scope:

- Add a default-on `desktop` cargo feature that owns `eframe`, `egui`, `image`,
  the `ui` module and the `screenshot` module.
- Declare the binary and the four UI test crates with
  `required-features = ["desktop"]`, so a core test build skips them.
- Run the quality gate as two passes: the core suite without the desktop stack,
  then the UI test crates with it.
- Reduce debug info: `debug = "line-tables-only"` for `dev` and `debug = 0` for
  `test`. Incremental compilation stays enabled and Cargo keeps its default
  parallelism.
- Add feature-seam tests and update the docs, spec index and changelog.

Out of scope:

- No `-j` cap, no `CARGO_INCREMENTAL=0`, no change to test execution threads.
- No new crate or Cargo workspace; ADR-004 still applies.
- No merge of the 33 test crates; that is a separate decision.
- No behavior, protocol, persistence, DTO or visual change.
- The UI/client split originally queued as `spec-048` moves to `spec-049`.

## Evidence

Measured on the development machine on 2026-11-10:

- 33 test crates and 253 tests; test executables are 12-16 MB each and their
  PDBs are 220-270 MB each; the binary PDB is 367 MB.
- `target/` was 72.2 GB: 48.9 GB of PDBs, 11.8 GB of incremental state and
  270 executables (including stale duplicates).
- `cargo test` defaults to 12 jobs on the 6-core/12-thread CPU. The machine has
  16 GB of RAM, 19/39 GB commit charge at idle, and a 23.7 GB page file that
  had already peaked at 12.2 GB.
- `eframe` 0.37's default features pull in `wgpu`, `naga` and `egui-wgpu` in
  addition to the app's direct `egui` use.
- Only `tests/ui.rs`, `tests/fidelity.rs`, `tests/ui_crdt.rs` and
  `tests/screenshot.rs` import the UI; the other 29 test crates are core.
- Test execution itself is cheap: 0.03-1.7 s per binary. The cost is compile
  and link, not the tests.
- The existing suite, clippy, fmt and `--screenshot` are the behavior contract.
- `docs/adr-004-modularization-decision.md` prefers one crate until a second
  bounded context appears; a feature seam is the smallest structural change
  that removes the desktop stack from core tests.

## Acceptance criteria

1. **Given** `--no-default-features`, **when** the crate is built, **then** it
   compiles without `eframe`, `egui`, `image`, `ui` or `screenshot`, and the
   core modules (`model`, `store`, `server`, `client`, `auth`, `collab`,
   `remote`, `cli`, `app`) are available.
2. **Given** `--no-default-features`, **when**
   `cargo test --all-targets` runs, **then** only the core test crates run; the
   binary and the four UI test crates are skipped through `required-features`.
3. **Given** the default `desktop` feature, **when** the app is built, run with
   `--screenshot`, or the UI test crates run, **then** behavior and visuals are
   unchanged.
4. **Given** the `test` profile, **when** tests are built, **then** test
   binaries and their dependencies carry no full debug info, so the ~250 MB
   PDBs are gone (the remaining per-test PDBs are about 4-15 MB); incremental
   compilation and Cargo's default job count are unchanged.
5. **Given** the `dev` profile, **when** the app is built, **then** debug info
   is line-tables-only and incremental compilation stays enabled.
6. **Given** the full gate, **when** `quality.ps1` runs, **then** it executes
   the core pass and the desktop pass, reports the combined passing total, and
   fails if either pass fails.
7. **Given** the docs, **when** the spec completes, **then** `AGENTS.md`,
   `docs/architecture.md`, `docs/testing.md`, `docs/requirements.md`,
   `docs/review-findings.md`, ADR-004, the spec index and the changelog
   describe the seam and the `spec-049` renumbering.

## Test scenarios (written before implementation)

- **Red step:** make `eframe`, `egui` and `image` optional and add the feature
  and `required-features` declarations, but leave `src/lib.rs` ungated. Run
  `cargo test --all-targets --no-default-features` and observe the unresolved
  `egui`/`image` compile failure.
- **Green core:** the same command passes and reports only the core crates.
- **Green desktop:**
  `cargo test --features desktop --test ui --test fidelity --test ui_crdt --test screenshot`
  passes.
- **Feature seam:**
  - `tests/app.rs` gets a `#[cfg(not(feature = "desktop"))]` test that uses
    `app::build_server` and `WorkspaceServer` as `WorkspaceApi`.
  - `tests/ui.rs` gets a `#[cfg(feature = "desktop")]` test that references
    `screenshot::color_image_to_rgba`.
- **Lint and format:** `cargo clippy --all-targets -- -D warnings`,
  `cargo clippy --all-targets --no-default-features -- -D warnings` and
  `cargo fmt --check` are clean.
- **Visual:** `cargo run -- --screenshot shots/now.png` still writes a
  non-blank PNG.
- **Build check:** `cargo tree --no-default-features` contains no
  `eframe`/`egui`/`wgpu`, and newly built test binaries have no 220-270 MB PDB.

## Design

```text
Cargo.toml
  [features]
  default = ["desktop"]
  desktop = ["dep:eframe", "dep:egui", "dep:image"]

  eframe/egui/image are optional dependencies
  [[bin]] ion-of-notes            required-features = ["desktop"]
  [[test]] ui/fidelity/ui_crdt/screenshot  required-features = ["desktop"]

  [profile.dev]  debug = "line-tables-only"   # incremental stays true
  [profile.test] debug = 0                    # incremental stays true

src/lib.rs
  #[cfg(feature = "desktop")] pub mod screenshot;
  #[cfg(feature = "desktop")] pub mod ui;

scripts/check-tests.ps1
  core pass:    cargo test --all-targets --no-default-features
  desktop pass: cargo test --features desktop --test ui --test fidelity \
                --test ui_crdt --test screenshot
  sum "test result: ok. N passed" from both passes
```

The core pass compiles the library and 29 test crates without the desktop
stack. The desktop pass compiles the desktop library once and the four UI test
crates. No `-j` or `CARGO_INCREMENTAL` override is added, so the user keeps
maximum parallelism and incremental builds.

## Plan / tasks

- [x] Write this spec and record the red compile step.
- [x] Make `eframe`, `egui` and `image` optional; add the `desktop` feature,
      the binary `required-features` and the four test `required-features`.
- [x] Gate `screenshot` and `ui` in `src/lib.rs`; keep default behavior.
- [x] Add the feature-seam tests.
- [x] Add the `dev`/`test` debug settings.
- [x] Rewrite `scripts/check-tests.ps1` as the core + desktop two-pass gate.
- [x] Run core tests, desktop tests, both clippy passes, fmt and `--screenshot`;
      verify the 220-270 MB test PDBs are gone (the remaining ones are 4-15 MB).
- [x] Update `AGENTS.md`, architecture, testing, requirements, review findings,
      ADR-004, spec-046/047 notes, spec index and changelog.
- [x] Commit the whole `app/` tree naming this spec.
