# spec-053 - Supply-chain and tooling baseline

- Status: implemented (2026-11-11)
- Related: `spec-052`, `.github/workflows/ci.yml`, `.gitlab-ci.yml`,
  `Dockerfile`, `docs/contributor-guide.md`

## Purpose and scope

The CI pipelines build and test the project, but the toolchain and third-party
inputs are mostly floating tags. This spec pins the inputs, adds an advisory
check and gives both CI systems the same supply-chain baseline.

In scope:

- Add `rust-toolchain.toml` pinning Rust 1.99.1 with `rustfmt` and `clippy`.
- Pin GitHub Actions by commit SHA and Docker/GitLab base images by digest.
- Add a `security` job running `cargo audit` in both GitHub Actions and GitLab
  CI.
- Add `renovate.json` for Cargo, GitHub Actions and Docker updates.
- Add `.editorconfig` for consistent indentation and line endings.
- Update the contributor guide, README, architecture, testing, requirements,
  glossary, spec index and changelog.

Out of scope:

- No `cargo deny` license/bans policy in this spec.
- No sccache, nextest or target-cache change.
- No product behavior, protocol, DTO or visual change.
- No new test count.

## Evidence

- `.github/workflows/ci.yml` uses `actions/checkout@v4`,
  `dtolnay/rust-toolchain@stable` and `Swatinem/rust-cache@v2` as moving tags.
- `.gitlab-ci.yml` uses `rust:1.99-bookworm` by tag, and `Dockerfile` uses
  `rust:1.99-bookworm` and `debian:bookworm-slim` by tag.
- `spec-052` added the GitHub container smoke job, which is another consumer of
  the image digests.
- The project has no advisory check, no automated dependency updates and no
  editor baseline.
- The existing gate already covers formatting, clippy, tests, docs and the
  preview screenshots; this spec adds only the supply-chain checks.

## Acceptance criteria

1. **Given** the repository, **when** `rustup show` runs, **then** the pinned
   `1.99.1` toolchain with `rustfmt` and `clippy` is selected.
2. **Given** the GitHub workflow, **when** it runs, **then** every action is
   referenced by a commit SHA with a version comment, and the Rust setup uses
   the pinned toolchain.
3. **Given** the Dockerfile and GitLab CI, **when** they build, **then** the
   base images are referenced by digest.
4. **Given** a vulnerable dependency, **when** the `security` job runs, **then**
   `cargo audit` fails the pipeline.
5. **Given** `renovate.json`, **when** Renovate runs, **then** Cargo, GitHub
   Actions and Docker updates are grouped and proposed.
6. **Given** `.editorconfig`, **when** an editor saves a file, **then** it uses
   the documented indentation and line endings.
7. **Given** the full gate, **when** fmt, both clippy passes, core + desktop
   tests, docs and the preview screenshots run, **then** all are green and the
   test count stays 258.

## Test scenarios (written before implementation)

- **Toolchain:** `rustup show` reports `1.99.1`; `cargo fmt --check` and clippy
  still pass.
- **Workflow syntax:** parse `.github/workflows/ci.yml` and `.gitlab-ci.yml`;
  verify no floating `@v` action references and no tag-only base image in the
  build files.
- **Advisory job:** run `cargo audit` locally after installing the tool; the
  current lockfile must be clean.
- **Renovate:** `renovate.json` parses and lists the cargo, github-actions and
  docker managers.
- **Full gate:** `pwsh -File scripts/quality.ps1` and the preview screenshots
  pass.

## Design

```text
rust-toolchain.toml        channel 1.99.1, rustfmt + clippy
.github/workflows/ci.yml   SHA-pinned actions, security job (cargo audit)
.gitlab-ci.yml             digest-pinned image, security job (cargo audit)
Dockerfile                 digest-pinned builder and runtime images
renovate.json              Cargo + GitHub Actions + Docker updates
.editorconfig              UTF-8, LF, 4-space Rust, 2-space YAML/Markdown
```

The advisory job is separate from the quality gate so a RustSec database
failure or a new advisory is visible without slowing every local test run. The
GitHub job uses `rustsec/audit-check`; the GitLab job installs `cargo-audit`
and runs it on merge requests and the default branch. Renovate updates the
pins, so pinning does not mean freezing.

## Plan / tasks

- [x] Write this spec and record the checks.
- [x] Add `rust-toolchain.toml` and `.editorconfig`.
- [x] Pin GitHub Actions and Docker/GitLab images.
- [x] Add the GitHub and GitLab `security` jobs.
- [x] Add `renovate.json`.
- [x] Run `rustup show`, `cargo audit`, YAML parsing and the full gate.
- [x] Update AGENTS, README, architecture, testing, requirements, glossary,
      spec index and changelog.
- [x] Commit the whole `app/` tree naming this spec.
