# Run all Markdown checks
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
$scripts = @(
    "check-links.ps1",
    "check-mermaid.ps1",
    "check-tables.ps1"
)

$failed = $false
foreach ($script in $scripts) {
    Write-Host "== $script =="
    & (Join-Path $PSScriptRoot $script) -Root $Root
    if ($LASTEXITCODE -ne 0) {
        $failed = $true
    }
}

if ($failed) {
    Write-Host "Documentation checks failed."
    exit 1
}

Write-Host "All documentation checks passed."
exit 0
