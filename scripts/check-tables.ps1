# Check Markdown table column consistency
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
$files = Get-ChildItem -Path $Root -Recurse -File -Filter *.md |
    Where-Object { $_.FullName -notmatch '[\\/](target|\.git|node_modules)[\\/]' }

$issues = @()

foreach ($file in $files) {
    $lines = Get-Content $file.FullName
    $inTable = $false
    $expected = 0

    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        if ($line -match '^\s*\|.*\|\s*$') {
            $clean = $line -replace '\\\|', ''
            $pipes = ([regex]::Matches($clean, '\|')).Count
            if (-not $inTable) {
                $inTable = $true
                $expected = $pipes
            } elseif ($pipes -ne $expected) {
                $issues += "{0}:{1}: {2} pipes, expected {3}" -f $file.FullName.Replace($Root + '\', ''), ($i + 1), $pipes, $expected
            }
        } else {
            $inTable = $false
        }
    }
}

if ($issues.Count -eq 0) {
    Write-Host "All Markdown tables have consistent column counts."
    exit 0
}

$issues | ForEach-Object { Write-Host "TABLE: $_" }
Write-Host "Table issues: $($issues.Count)"
exit 1
