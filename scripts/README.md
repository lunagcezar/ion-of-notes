# Scripts

Small PowerShell checks used by contributors and agents. They are safe to run
from the repository root and do not modify files.

| Script | Purpose |
| ------ | ------- |
| `check-links.ps1` | Every relative Markdown link resolves to an existing file. |
| `check-mermaid.ps1` | Every Mermaid fence is closed and starts with a recognized diagram type. |
| `check-tables.ps1` | Every Markdown table has a consistent number of columns. |
| `check-docs.ps1` | Runs the three documentation checks above. |
| `check-tests.ps1` | Runs `cargo test --all-targets` and prints the passing-test total. |
| `quality.ps1` | Full gate: `cargo fmt --check`, clippy, tests and documentation checks. |

## Usage

From the repository root:

```powershell
pwsh -File scripts/quality.ps1
pwsh -File scripts/check-docs.ps1
pwsh -File scripts/check-tests.ps1
pwsh -File scripts/check-tests.ps1 -VerboseOutput
```

Each script exits `0` on success and non-zero on failure, so it works in CI or
a pre-commit hook.

## Notes

- The Markdown checks ignore `target/`, `.git/` and `node_modules/`.
- `check-tests.ps1` prints only the summary by default; use `-VerboseOutput` to
  see the full `cargo test` output.
- `quality.ps1` runs the same commands documented in `AGENTS.md` and
  `docs/testing.md`.
