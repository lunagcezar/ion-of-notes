# Check Mermaid fences and diagram types
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
$files = Get-ChildItem -Path $Root -Recurse -File -Filter *.md |
    Where-Object { $_.FullName -notmatch '[\\/](target|\.git|node_modules)[\\/]' }

$allowed = '^(flowchart|graph|sequenceDiagram|classDiagram|erDiagram|stateDiagram|gantt|pie|journey|gitGraph|mindmap|timeline|quadrantChart|requirementDiagram|C4Context)'
$issues = @()

foreach ($file in $files) {
    $lines = Get-Content $file.FullName
    $inMermaid = $false
    $startLine = 0
    $firstLine = ''

    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        if ($line -match '^\s*```mermaid\s*$') {
            $inMermaid = $true
            $startLine = $i + 1
            $firstLine = ''
            continue
        }
        if ($inMermaid -and $line -match '^\s*```\s*$') {
            if ($firstLine -notmatch $allowed) {
                $issues += "{0}:{1}: unrecognized Mermaid type '{2}'" -f $file.FullName.Replace($Root + '\', ''), $startLine, $firstLine
            }
            $inMermaid = $false
            continue
        }
        if ($inMermaid -and $firstLine -eq '' -and $line.Trim() -ne '') {
            $firstLine = $line.Trim()
        }
    }

    if ($inMermaid) {
        $issues += "{0}: unclosed Mermaid fence at line {1}" -f $file.FullName.Replace($Root + '\', ''), $startLine
    }
}

if ($issues.Count -eq 0) {
    Write-Host "All Mermaid blocks are valid and closed."
    exit 0
}

$issues | ForEach-Object { Write-Host "MERMAID: $_" }
Write-Host "Mermaid issues: $($issues.Count)"
exit 1
