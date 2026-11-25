# spec-149 - Release checksums

- Status: implemented (2026-11-25)
- Related: `spec-130`, `docs/deployment.md`
- Review source: 2026-11-25 Kimi review, release supply-chain note

## Purpose and scope

Tagged releases published three archives but no checksums, so a download could
not be verified against the published artifact.

### Workflow

A final `checksums` job runs after `linux-server` and `windows`:

1. downloads every asset of the tag into `dist/`;
2. computes `sha256sum` over each file, sorted;
3. writes `SHA256SUMS`;
4. uploads it with `gh release upload --clobber`.

`SHA256SUMS` is deterministic for a given set of assets and uses the standard
`hash  filename` format, so `sha256sum -c SHA256SUMS` works on Linux/macOS and
`Get-FileHash`/WSL on Windows.

Signing (Sigstore/cosign) remains a separate step: it needs keyless OIDC
permissions and is documented as future work.

## Acceptance criteria

P1. **Given** a `v*` tag, **then** the workflow waits for both platform jobs.

P2. **Given** the assets, **then** a `SHA256SUMS` file contains one line per
archive.

P3. **Given** a re-run, **then** the checksum asset is replaced, not duplicated.

## Test scenarios

- `tests/deployment.rs`
  (`release_workflow_publishes_checksums_after_both_platforms`).
- `pwsh -File scripts/check-docs.ps1`.

## Plan / tasks

- [x] Add the `checksums` job with `needs`.
- [x] Generate and upload `SHA256SUMS`.
- [x] Document verification in `docs/deployment.md`.
- [x] Add the drift test.
