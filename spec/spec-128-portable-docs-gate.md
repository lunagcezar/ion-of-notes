# spec-128 - PowerShell-free documentation and quality gate

- Status: implemented (2026-11-24)
- Related: `spec-029`, `spec-048`, `docs/index.md`
- Review source: deferred PowerShell-free gate in `docs/review-response.md`

## Purpose and scope

The quality documentation checks (`check-links.ps1`, `check-mermaid.ps1`,
`check-tables.ps1`) required PowerShell. Contributors on Unix or minimal CI
images could not run the full gate.

This spec adds:

- `tests/docs.rs`, a Rust integration test that walks every Markdown file
  (ignoring `target/`, `.git/`, `node_modules/`) and checks:
  - relative links resolve;
  - table rows keep a consistent unescaped pipe count;
  - Mermaid fences close and start with a recognized diagram type;
- `scripts/quality.sh`, a POSIX shell wrapper for fmt, clippy (both feature
  passes), core tests, desktop UI tests and the full default test pass;
- documentation checks now run inside `cargo test`, so every feature pass
  includes them.

The PowerShell scripts remain as a fast pre-check; they are no longer the only
way to verify documentation.

Out of scope:

- removing the PowerShell scripts;
- a Markdown parser dependency;
- remote link checking.

## Acceptance criteria

P1. **Given** a broken relative link, **then** `tests/docs.rs` fails with the
file, line and target.

P2. **Given** an inconsistent table row, **then** the test fails with the
expected pipe count.

P3. **Given** an unclosed or unknown Mermaid block, **then** the test fails.

P4. **Given** `sh scripts/quality.sh`, **then** it runs the same commands as
the PowerShell gate without requiring `pwsh`.

## Test scenarios

- `cargo test --no-default-features --test docs`.
- `sh scripts/quality.sh` (POSIX shell).
- Existing `pwsh -File scripts/quality.ps1`.
- Full quality gate.

## Design

| Piece | Location |
| ----- | -------- |
| Portable Markdown checks | `tests/docs.rs` |
| POSIX gate | `scripts/quality.sh` |

## Plan / tasks

- [x] Port link, table and Mermaid checks to Rust.
- [x] Run the checks in the normal test passes.
- [x] Add the POSIX quality wrapper.
- [x] Keep the PowerShell scripts as a pre-check.
