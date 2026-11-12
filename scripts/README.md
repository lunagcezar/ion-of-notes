# Scripts

Small PowerShell checks used by contributors and agents. They are safe to run
from the repository root and do not modify files.

| Script | Purpose |
| ------ | ------- |
| `check-links.ps1` | Every relative Markdown link resolves to an existing file. |
| `check-mermaid.ps1` | Every Mermaid fence is closed and starts with a recognized diagram type. |
| `check-tables.ps1` | Every Markdown table has a consistent number of columns. |
| `check-docs.ps1` | Runs the three documentation checks above. |
| `check-tests.ps1` | Runs the core tests without the desktop stack, then the desktop UI tests, and prints the combined total. |
| `quality.ps1` | Full gate: `cargo fmt --check`, clippy, core + desktop tests and documentation checks. |
| `smoke-server.ps1` | Builds/runs the headless server with a temporary SQLite database and checks health/ready/metrics/v2/page. |
| `smoke-docker.ps1` | Opt-in Compose smoke: build, health, page write/read and volume persistence across recreate. |

## Usage

From the repository root:

```powershell
pwsh -File scripts/quality.ps1
pwsh -File scripts/check-docs.ps1
pwsh -File scripts/check-tests.ps1
pwsh -File scripts/check-tests.ps1 -VerboseOutput
pwsh -File scripts/smoke-server.ps1
pwsh -File scripts/smoke-docker.ps1
```

Each script exits `0` on success and non-zero on failure, so it works in CI or
a pre-commit hook.

## Notes

- The Markdown checks ignore `target/`, `.git/` and `node_modules/`.
- `check-tests.ps1` runs `cargo test --all-targets --no-default-features`
  (251 core tests) and then the four UI test crates with `--features desktop`
  (35 tests). It prints only the summary by default; use `-VerboseOutput` to
  see the full `cargo test` output.
- `smoke-server.ps1` and `smoke-docker.ps1` are opt-in runtime smokes; the
  Docker one builds an image and uses an isolated smoke volume.
- No script caps Cargo jobs or disables incremental compilation; spec-048 keeps
  both at their defaults.
- `quality.ps1` runs the same commands documented in `AGENTS.md` and
  `docs/testing.md`, and is used by both `.github/workflows/ci.yml` and
  `.gitlab-ci.yml`.
