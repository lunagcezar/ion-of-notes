# Runtime smoke test for the headless server binary (spec-067).
#
# Builds the server-only binary, starts it on a free loopback port with a
# temporary SQLite database, checks the operational endpoints, writes and reads
# a page through the API, then always stops the process and removes the temp
# directory.

param(
    [string]$Bind = "127.0.0.1",
    [int]$Port = 0,
    [int]$TimeoutSec = 45,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
$process = $null
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("ion-smoke-" + [guid]::NewGuid().ToString("N"))
$oldPassword = $env:ION_OF_NOTES_OWNER_PASSWORD
$oldPort = $env:ION_OF_NOTES_PORT
$oldBind = $env:ION_OF_NOTES_BIND

function Fail([string]$message) {
    Write-Host "SMOKE FAILED: $message" -ForegroundColor Red
    exit 1
}

function Wait-Http([string]$url, [int]$seconds) {
    $deadline = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $deadline) {
        try {
            $response = Invoke-WebRequest -UseBasicParsing -Uri $url -TimeoutSec 3
            if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 300) {
                return $response
            }
        } catch {
            Start-Sleep -Milliseconds 250
        }
    }
    return $null
}

try {
    New-Item -ItemType Directory -Force -Path $temp | Out-Null
    if (-not $SkipBuild) {
        Write-Host "== build headless server =="
        & cargo build --release --locked --no-default-features --bin ion-of-notes-server
        if ($LASTEXITCODE -ne 0) { Fail "cargo build failed" }
    }
    $binary = Join-Path $root "target/release/ion-of-notes-server.exe"
    if (-not (Test-Path $binary)) { $binary = Join-Path $root "target/release/ion-of-notes-server" }
    if (-not (Test-Path $binary)) { Fail "server binary not found; run without -SkipBuild" }

    if ($Port -eq 0) {
        $probe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
        $probe.Start()
        $Port = ([System.Net.IPEndPoint]$probe.LocalEndpoint).Port
        $probe.Stop()
    }
    $base = "http://${Bind}:${Port}"
    $database = Join-Path $temp "smoke.sqlite3"
    $env:ION_OF_NOTES_OWNER_PASSWORD = "smoke-owner-password"
    $env:ION_OF_NOTES_PORT = "$Port"
    $env:ION_OF_NOTES_BIND = $Bind

    Write-Host "== start server on $base =="
    $process = Start-Process -FilePath $binary `
        -ArgumentList @("--serve", "${Bind}:${Port}", "--database", $database) `
        -PassThru -NoNewWindow
    $health = Wait-Http "$base/health" $TimeoutSec
    if ($null -eq $health) { Fail "server did not answer /health" }

    $ready = Wait-Http "$base/ready" $TimeoutSec
    if ($null -eq $ready) { Fail "server did not answer /ready" }
    if ($ready.Content -notmatch "ready") { Fail "/ready body did not contain ready" }

    $metrics = Invoke-WebRequest -UseBasicParsing -Uri "$base/metrics"
    if ($metrics.Content -notmatch "ion_of_notes_up 1") { Fail "/metrics missing ion_of_notes_up" }

    $headers = @{
        "ion-of-notes-version" = "2026-11-08"
        "Authorization"        = "Bearer owner-token"
    }
    $workspace = Invoke-RestMethod -Uri "$base/api/v2/workspace" -Headers $headers
    if ($null -eq $workspace.data) { Fail "v2 workspace response has no data" }

    Write-Host "== create and read a page =="
    $draft = @{ title = "Smoke page"; icon = ""; parent = $null } | ConvertTo-Json
    $created = Invoke-RestMethod -Method Post -Uri "$base/api/pages" -ContentType "application/json" -Body $draft
    if ($null -eq $created.data.id) { Fail "page create response has no id" }
    $page = Invoke-RestMethod -Uri "$base/api/pages/$($created.data.id)"
    if ($page.data.title -ne "Smoke page") { Fail "page read did not return the created title" }

    Write-Host "SMOKE PASSED: $base" -ForegroundColor Green
    exit 0
} catch {
    $detail = $_.ErrorDetails.Message
    if ($detail) { Fail ($_.Exception.Message + " :: " + $detail) }
    Fail $_.Exception.Message
} finally {
    if ($null -ne $process -and -not $process.HasExited) {
        try { Stop-Process -Id $process.Id -Force } catch {}
        $process.WaitForExit(5000) | Out-Null
    }
    $env:ION_OF_NOTES_OWNER_PASSWORD = $oldPassword
    $env:ION_OF_NOTES_PORT = $oldPort
    $env:ION_OF_NOTES_BIND = $oldBind
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $temp
    Pop-Location
}
