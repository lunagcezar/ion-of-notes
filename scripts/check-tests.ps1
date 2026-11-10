# Run the core suite without the desktop stack, then the UI test crates with it.
param(
    [string]$Root = (Split-Path -Parent $PSScriptRoot),
    [switch]$VerboseOutput
)

$ErrorActionPreference = "Stop"
Push-Location $Root
try {
    # Core tests build without eframe/egui/wgpu; the four UI crates need the
    # default-on `desktop` feature. No `-j` or CARGO_INCREMENTAL override is
    # used: Cargo keeps its default parallelism and incremental compilation.
    $passes = @(
        @{
            Name = "core"
            Args = @("test", "--all-targets", "--no-default-features")
        },
        @{
            Name = "desktop"
            Args = @(
                "test", "--features", "desktop",
                "--test", "ui",
                "--test", "fidelity",
                "--test", "ui_crdt",
                "--test", "screenshot"
            )
        }
    )

    $total = 0
    foreach ($pass in $passes) {
        Write-Host ("== cargo " + ($pass.Args -join " ") + " ==")
        $output = & cargo @($pass.Args) 2>&1
        $exitCode = $LASTEXITCODE
        if ($exitCode -ne 0 -or $VerboseOutput) {
            $output | ForEach-Object { Write-Host $_ }
        }
        if ($exitCode -ne 0) {
            Write-Host ("cargo test ({0}) failed." -f $pass.Name)
            exit $exitCode
        }

        $passed = 0
        foreach ($line in $output) {
            if ($line -match 'test result: ok\. (\d+) passed') {
                $passed += [int]$Matches[1]
            }
        }
        $total += $passed
        Write-Host ("{0} pass: {1} passed" -f $pass.Name, $passed)
    }

    Write-Host "cargo test: $total passed (core + desktop)"
    exit 0
} finally {
    Pop-Location
}
