# Check relative Markdown links
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
$files = Get-ChildItem -Path $Root -Recurse -File -Filter *.md |
    Where-Object { $_.FullName -notmatch '[\\/](target|\.git|node_modules)[\\/]' }

$missing = @()
foreach ($file in $files) {
    $text = Get-Content -Raw $file.FullName
    foreach ($match in [regex]::Matches($text, '\]\(([^)]+)\)')) {
        $target = $match.Groups[1].Value.Trim()
        if ($target -match '^(https?:|mailto:|#)') { continue }
        $target = $target.Split('#')[0]
        if ([string]::IsNullOrWhiteSpace($target)) { continue }
        $resolved = Join-Path $file.DirectoryName $target
        if (-not (Test-Path -LiteralPath $resolved)) {
            $missing += "{0} -> {1}" -f $file.FullName.Replace($Root + '\', ''), $target
        }
    }
}

if ($missing.Count -eq 0) {
    Write-Host "All relative links resolve."
    exit 0
}

$missing | ForEach-Object { Write-Host "MISSING: $_" }
Write-Host "Broken relative links: $($missing.Count)"
exit 1
