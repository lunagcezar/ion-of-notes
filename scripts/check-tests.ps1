# Run the integration suite and report the total number of passing tests
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot),
    [switch]$VerboseOutput
)

$ErrorActionPreference = "Stop"
Push-Location $Root
try {
    $output = & cargo test --all-targets 2>&1
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0 -or $VerboseOutput) {
        $output | ForEach-Object { Write-Host $_ }
    }
    if ($exitCode -ne 0) {
        Write-Host "cargo test --all-targets failed."
        exit $exitCode
    }

    $total = 0
    foreach ($line in $output) {
        if ($line -match 'test result: ok\. (\d+) passed') {
            $total += [int]$Matches[1]
        }
    }
    Write-Host "cargo test --all-targets: $total passed"
    exit 0
} finally {
    Pop-Location
}
