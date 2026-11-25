# Opt-in PostgreSQL + Redis smoke test (spec-137).
#
# Starts disposable PostgreSQL and Redis containers, waits for readiness, runs
# the opt-in integration tests against them and removes the containers again.
# Requires Docker. Nothing is written to the repository or to a host volume.

param(
    [int]$PostgresPort = 15432,
    [int]$RedisPort = 16379,
    [int]$TimeoutSec = 60
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
$postgres = "ion-of-notes-smoke-pg"
$redis = "ion-of-notes-smoke-redis"
$password = "smoke-password"
$database = "ion_smoke"

function Fail([string]$message) {
    Write-Host "EXTERNAL SMOKE FAILED: $message" -ForegroundColor Red
    exit 1
}

function Wait-Container([string]$name, [string]$command, [int]$seconds) {
    $deadline = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $deadline) {
        docker exec $name $command.Split(" ") 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) { return }
        Start-Sleep -Milliseconds 500
    }
    Fail "$name did not become ready"
}

try {
    docker run -d --name $postgres `
        -e POSTGRES_PASSWORD=$password `
        -e POSTGRES_DB=$database `
        -p "${PostgresPort}:5432" `
        postgres:16-alpine | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail "could not start PostgreSQL" }

    docker run -d --name $redis -p "${RedisPort}:6379" redis:7-alpine | Out-Null
    if ($LASTEXITCODE -ne 0) { Fail "could not start Redis" }

    Wait-Container $postgres "pg_isready -U postgres" $TimeoutSec
    Wait-Container $redis "redis-cli ping" $TimeoutSec

    $env:ION_OF_NOTES_POSTGRES_TEST_URL =
        "postgres://postgres:${password}@127.0.0.1:${PostgresPort}/${database}"
    $env:ION_OF_NOTES_REDIS_TEST_URL = "redis://127.0.0.1:${RedisPort}"

    cargo test --no-default-features --features postgres,redis `
        --test postgres_store --test redis_limits
    if ($LASTEXITCODE -ne 0) { Fail "integration tests failed" }

    Write-Host "External PostgreSQL/Redis smoke passed." -ForegroundColor Green
}
finally {
    docker rm -f $postgres $redis 2>$null | Out-Null
    Pop-Location
}
