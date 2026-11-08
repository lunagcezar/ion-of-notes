# Full quality gate: formatting, clippy, tests and Markdown checks
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
Push-Location $Root
try {
    Write-Host "== cargo fmt --check =="
    & cargo fmt --check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "== cargo clippy --all-targets -- -D warnings =="
    & cargo clippy --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "== tests =="
    & (Join-Path $PSScriptRoot "check-tests.ps1") -Root $Root
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "== docs =="
    & (Join-Path $PSScriptRoot "check-docs.ps1") -Root $Root
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    Write-Host "All checks passed."
    exit 0
} finally {
    Pop-Location
}
