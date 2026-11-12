# Opt-in Docker Compose smoke test (spec-067).
#
# Builds the development stack, waits for health, checks /health, /ready,
# /metrics and the v2 workspace, writes/reads a page, recreates the service and
# verifies the page survives in an isolated smoke volume. Requires Docker.

param(
    [int]$Port = 0,
    [int]$TimeoutSec = 90
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
$override = Join-Path ([System.IO.Path]::GetTempPath()) ("ion-compose-smoke-" + [guid]::NewGuid().ToString("N") + ".yml")
$secret = Join-Path $root "secrets/owner_password.txt"
$createdSecret = $false
$project = "ion-of-notes-smoke"
$oldPort = $env:ION_OF_NOTES_PORT
$oldBind = $env:ION_OF_NOTES_BIND

function Fail([string]$message) {
    Write-Host "DOCKER SMOKE FAILED: $message" -ForegroundColor Red
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
            Start-Sleep -Milliseconds 500
        }
    }
    return $null
}

try {
    & docker info *> $null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Docker is not running; the Docker smoke test is opt-in and was skipped." -ForegroundColor Yellow
        exit 0
    }

    if ($Port -eq 0) {
        $probe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
        $probe.Start()
        $Port = ([System.Net.IPEndPoint]$probe.LocalEndpoint).Port
        $probe.Stop()
    }
    $base = "http://127.0.0.1:${Port}"
    $env:ION_OF_NOTES_PORT = "$Port"
    $env:ION_OF_NOTES_BIND = "127.0.0.1"

    if (-not (Test-Path $secret)) {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $secret) | Out-Null
        Set-Content -Path $secret -Value ("smoke-" + [guid]::NewGuid().ToString("N"))
        $createdSecret = $true
    }

    @"
services:
  ion-of-notes:
    ports:
      - "127.0.0.1:${Port}:8787"
volumes:
  ion-data:
    name: ion-of-notes-smoke-data
"@ | Set-Content -Path $override

    Write-Host "== docker compose build and start =="
    & docker compose -p $project -f compose.yml -f $override up -d --build
    if ($LASTEXITCODE -ne 0) { Fail "docker compose up failed" }

    $health = Wait-Http "$base/health" $TimeoutSec
    if ($null -eq $health) { Fail "container did not answer /health" }
    $ready = Wait-Http "$base/ready" $TimeoutSec
    if ($null -eq $ready) { Fail "container did not answer /ready" }
    $metrics = Invoke-WebRequest -UseBasicParsing -Uri "$base/metrics"
    if ($metrics.Content -notmatch "ion_of_notes_up 1") { Fail "/metrics missing ion_of_notes_up" }

    $headers = @{
        "ion-of-notes-version" = "2026-11-08"
        "Authorization"        = "Bearer owner-token"
    }
    $workspace = Invoke-RestMethod -Uri "$base/api/v2/workspace" -Headers $headers
    if ($null -eq $workspace.data) { Fail "v2 workspace response has no data" }

    Write-Host "== create and read a page =="
    $draft = @{ title = "Docker smoke page"; icon = ""; parent = $null } | ConvertTo-Json
    $created = Invoke-RestMethod -Method Post -Uri "$base/api/pages" -ContentType "application/json" -Body $draft
    if ($null -eq $created.data.id) { Fail "page create response has no id" }
    $page = Invoke-RestMethod -Uri "$base/api/pages/$($created.data.id)"
    if ($page.data.title -ne "Docker smoke page") { Fail "page read did not return the created title" }

    Write-Host "== recreate and verify the volume =="
    & docker compose -p $project -f compose.yml -f $override up -d --force-recreate
    if ($LASTEXITCODE -ne 0) { Fail "docker compose recreate failed" }
    $health = Wait-Http "$base/health" $TimeoutSec
    if ($null -eq $health) { Fail "recreated container did not answer /health" }
    $page = Invoke-RestMethod -Uri "$base/api/pages/$($created.data.id)"
    if ($page.data.title -ne "Docker smoke page") { Fail "page did not survive the recreate" }

    Write-Host "DOCKER SMOKE PASSED: $base" -ForegroundColor Green
    exit 0
} catch {
    Fail $_.Exception.Message
} finally {
    try { & docker compose -p $project -f compose.yml -f $override down -v --remove-orphans *> $null } catch {}
    if ($createdSecret) { Remove-Item -Force -ErrorAction SilentlyContinue $secret }
    Remove-Item -Force -ErrorAction SilentlyContinue $override
    $env:ION_OF_NOTES_PORT = $oldPort
    $env:ION_OF_NOTES_BIND = $oldBind
    Pop-Location
}
