#Requires -Version 5.1
<#
.SYNOPSIS
    Govrix Scout — Windows PowerShell Installer

.DESCRIPTION
    Installs Govrix Scout on Windows using Docker.

    Two modes:
      Default (-Dev not set)  — Docker-only, no Rust or Node.js needed.
      -Dev                    — Full contributor setup (requires Rust + Node.js 20+).

.PARAMETER Dev
    Switch to enable full contributor / developer setup.

.EXAMPLE
    # End-user (Docker only):
    iwr -useb https://raw.githubusercontent.com/manaspros/govrix-scout/main/install.ps1 | iex

    # Contributor setup:
    .\install.ps1 -Dev

    # Pipe with flag (save first, then run):
    iwr -useb https://raw.githubusercontent.com/manaspros/govrix-scout/main/install.ps1 -OutFile install.ps1
    .\install.ps1 -Dev

.NOTES
    Requires Windows 10 (1903+) or Windows 11 with Docker Desktop installed.
    Run from PowerShell 5.1+ or PowerShell 7+.
#>

[CmdletBinding()]
param(
    [switch]$Dev,
    [string]$GovrixDir = "$env:USERPROFILE\.govrix"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ── Config ────────────────────────────────────────────────────────────────────
$REPO_RAW_BASE = "https://raw.githubusercontent.com/manaspros/govrix-scout/main"
$REPO_URL      = "https://github.com/manaspros/govrix-scout"

# ── Colors ────────────────────────────────────────────────────────────────────
function Write-Info    { param([string]$Msg) Write-Host "[govrix] $Msg" -ForegroundColor Cyan }
function Write-Success { param([string]$Msg) Write-Host "[govrix] $Msg" -ForegroundColor Green }
function Write-Warn    { param([string]$Msg) Write-Host "[govrix] WARN: $Msg" -ForegroundColor Yellow }
function Write-Fail    {
    param([string]$Msg)
    Write-Host "[govrix] ERROR: $Msg" -ForegroundColor Red
    exit 1
}

function Write-Separator { Write-Host ("=" * 63) }

# ── Secure random hex ─────────────────────────────────────────────────────────
function New-SecureHex {
    param([int]$ByteCount = 24)
    $rng   = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    $bytes = [byte[]]::new($ByteCount)
    $rng.GetBytes($bytes)
    $rng.Dispose()
    return ([System.BitConverter]::ToString($bytes) -replace '-', '').ToLower()
}

# ── Check Docker ──────────────────────────────────────────────────────────────
function Test-Docker {
    Write-Info "Checking Docker..."

    if (-not (Get-Command docker -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Fail @"
Docker is not installed.

  Install Docker Desktop from: https://docs.docker.com/get-docker/
  Then re-run this installer.
"@
    }

    $dockerInfo = docker info 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Fail @"
Docker daemon is not running.

  Start Docker Desktop, wait for it to be ready, then re-run this installer.
"@
    }

    $composeCheck = docker compose version 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Fail @"
Docker Compose v2 is not available.

  Upgrade Docker Desktop to include Compose v2.
  See: https://docs.docker.com/compose/install/
"@
    }

    $composeVer = (docker compose version --short 2>$null) -join ''
    if (-not $composeVer) { $composeVer = 'v2' }
    Write-Success "Docker ready  (Compose $composeVer)"
}

# ── Check Rust ────────────────────────────────────────────────────────────────
function Test-OrInstallRust {
    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        Write-Success "Rust found: $(rustc --version)"
        return
    }

    Write-Info "Rust not found — installing via rustup (this takes ~2 minutes)..."
    $rustupInstaller = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri 'https://win.rustup.rs/x86_64' -OutFile $rustupInstaller -UseBasicParsing
    & $rustupInstaller -y --default-toolchain stable
    Remove-Item $rustupInstaller -Force -ErrorAction SilentlyContinue

    # Refresh PATH for this session
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"

    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        Write-Success "Rust installed: $(rustc --version)"
    } else {
        Write-Fail "Rust installation failed. Please install manually from https://rustup.rs"
    }
}

# ── Check Node.js ─────────────────────────────────────────────────────────────
function Test-Node {
    if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Fail @"
Node.js 20+ is required for -Dev mode.

  Download from: https://nodejs.org
  Or install via winget:  winget install OpenJS.NodeJS.LTS
  Or install via nvm-windows: https://github.com/coreybutler/nvm-windows
"@
    }

    $nodeVer = (node --version) -replace '^v', ''
    $nodeMajor = [int]($nodeVer -split '\.')[0]
    if ($nodeMajor -lt 20) {
        Write-Fail "Node.js 20+ required, found v$nodeVer.`n  Update: winget upgrade OpenJS.NodeJS.LTS"
    }
    Write-Success "Node.js v$nodeVer"
}

# ── Check / install pnpm ──────────────────────────────────────────────────────
function Test-OrInstallPnpm {
    if (Get-Command pnpm -ErrorAction SilentlyContinue) {
        Write-Success "pnpm $(pnpm --version)"
        return
    }
    Write-Info "Installing pnpm..."
    npm install -g pnpm
    Write-Success "pnpm $(pnpm --version) installed"
}

# ── Health check loop ─────────────────────────────────────────────────────────
function Wait-Healthy {
    param(
        [string]$Url,
        [string]$Label,
        [int]   $TimeoutSec = 90,
        [int]   $IntervalSec = 3
    )

    Write-Host "[govrix] Waiting for $Label" -ForegroundColor Cyan -NoNewline
    $elapsed = 0

    while ($elapsed -lt $TimeoutSec) {
        try {
            $resp = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 2 -ErrorAction Stop
            if ($resp.StatusCode -eq 200) {
                Write-Host " done" -ForegroundColor Cyan
                Write-Success "$Label is ready"
                return $true
            }
        } catch {
            # Not ready yet — swallow the error
        }
        Write-Host "." -NoNewline
        Start-Sleep -Seconds $IntervalSec
        $elapsed += $IntervalSec
    }

    Write-Host ""
    Write-Warn "$Label did not become ready in ${TimeoutSec}s — check logs:`n  docker compose --project-directory `"$GovrixDir`" logs"
    return $false
}

# =============================================================================
# USER MODE — Docker-only, no Rust/Node needed
# =============================================================================
function Install-User {
    Write-Host ""
    Write-Host "Govrix — Quick Install (Docker)" -ForegroundColor White -BackgroundColor DarkBlue
    Write-Separator
    Write-Host ""

    Test-Docker

    # Create install directory
    if (-not (Test-Path $GovrixDir)) {
        New-Item -ItemType Directory -Path $GovrixDir -Force | Out-Null
    }
    Write-Info "Install directory: $GovrixDir"

    # Download docker-compose.yml
    $composeDest = Join-Path $GovrixDir "docker-compose.yml"
    if (Test-Path $composeDest) {
        Write-Info "docker-compose.yml already exists — keeping"
    } else {
        Write-Info "Downloading docker-compose.yml..."
        try {
            Invoke-WebRequest -Uri "$REPO_RAW_BASE/docker/docker-compose.yml" `
                              -OutFile $composeDest `
                              -UseBasicParsing
            Write-Success "docker-compose.yml downloaded"
        } catch {
            Write-Fail "Failed to download docker-compose.yml: $_"
        }
    }

    # Generate .env with secure random credentials
    $envFile = Join-Path $GovrixDir ".env"
    if (-not (Test-Path $envFile)) {
        Write-Info "Generating .env with secure credentials..."
        $dbPass    = New-SecureHex -ByteCount 24
        $timestamp = (Get-Date -Format "yyyy-MM-ddTHH:mm:ssZ" -AsUTC)

        $envContent = @"
# Govrix configuration — generated by install.ps1 $timestamp
# Edit this file to change ports, credentials, or retention settings.

POSTGRES_USER=govrix
POSTGRES_DB=govrix
POSTGRES_PASSWORD=$dbPass

GOVRIX_DATABASE__URL=postgres://govrix:${dbPass}@postgres:5432/govrix
GOVRIX_DATABASE__MAX_CONNECTIONS=20
GOVRIX_DATABASE__MIN_CONNECTIONS=2

GOVRIX_PROXY__LISTEN_ADDR=0.0.0.0:4000
GOVRIX_API__LISTEN_ADDR=0.0.0.0:4001
GOVRIX_METRICS__LISTEN_ADDR=0.0.0.0:9090

RUST_LOG=govrix_scout_proxy=info,tower_http=warn
"@
        Set-Content -Path $envFile -Value $envContent -Encoding UTF8
        Write-Success ".env created"
    } else {
        Write-Success ".env already exists — keeping existing credentials"
    }

    # Pull and start
    Write-Info "Pulling Docker images (first run takes 2-5 minutes)..."
    docker compose --project-directory $GovrixDir pull --quiet
    if ($LASTEXITCODE -ne 0) { Write-Fail "docker compose pull failed." }
    Write-Success "Images ready"

    Write-Info "Starting Govrix services..."
    docker compose --project-directory $GovrixDir up -d
    if ($LASTEXITCODE -ne 0) { Write-Fail "docker compose up failed." }
    Write-Success "Services started"

    # Health checks
    Write-Host ""
    Wait-Healthy -Url "http://localhost:4001/health" -Label "Govrix API"       -TimeoutSec 90 | Out-Null
    Wait-Healthy -Url "http://localhost:3000"         -Label "Govrix Dashboard" -TimeoutSec 30 | Out-Null

    # Read API key from .env (if present) for display
    $apiKey = ""
    if (Test-Path $envFile) {
        $apiKeyLine = Select-String -Path $envFile -Pattern '^GOVRIX_API_KEY=' | Select-Object -First 1
        if ($apiKeyLine) {
            $apiKey = ($apiKeyLine.Line -split '=', 2)[1]
        }
    }

    # Done — success summary
    Write-Host ""
    Write-Separator
    Write-Host "Govrix is running!" -ForegroundColor Green
    Write-Host ""
    Write-Host "  Dashboard:   " -NoNewline; Write-Host "http://localhost:3000"        -ForegroundColor Cyan
    Write-Host "  API:         " -NoNewline; Write-Host "http://localhost:4001/health" -ForegroundColor Cyan
    Write-Host "  Metrics:     " -NoNewline; Write-Host "http://localhost:9090/metrics" -ForegroundColor Cyan
    if ($apiKey) {
        Write-Host ""
        Write-Host "  API Key:     " -NoNewline; Write-Host $apiKey -ForegroundColor Yellow
    }
    Write-Host ""
    Write-Host "  Point your agents at Govrix (one env var, no code changes):" -ForegroundColor White
    Write-Host ""
    Write-Host '    $env:OPENAI_BASE_URL    = "http://localhost:4000/proxy/openai/v1"'
    Write-Host '    $env:ANTHROPIC_BASE_URL = "http://localhost:4000/proxy/anthropic/v1"'
    Write-Host ""
    Write-Host "  Manage:" -ForegroundColor White
    Write-Host "    docker compose --project-directory `"$GovrixDir`" logs -f"
    Write-Host "    docker compose --project-directory `"$GovrixDir`" down"
    Write-Host "    docker compose --project-directory `"$GovrixDir`" up -d"
    Write-Host ""
    Write-Host "  Docs:  $REPO_URL" -ForegroundColor White
    Write-Separator
    Write-Host ""
}

# =============================================================================
# DEV MODE — Full contributor setup
# =============================================================================
function Install-Dev {
    Write-Host ""
    Write-Host "Govrix — Developer Setup" -ForegroundColor White -BackgroundColor DarkBlue
    Write-Separator
    Write-Host ""

    # Locate repo root — must be run from inside the cloned repo
    $repoDir = $null
    if (Test-Path (Join-Path (Get-Location) "Cargo.toml")) {
        $cargoContent = Get-Content (Join-Path (Get-Location) "Cargo.toml") -Raw
        if ($cargoContent -match 'govrix-scout') {
            $repoDir = (Get-Location).Path
        }
    }
    if (-not $repoDir -and (Test-Path (Join-Path (Split-Path (Get-Location) -Parent) "Cargo.toml"))) {
        $parentCargo = Get-Content (Join-Path (Split-Path (Get-Location) -Parent) "Cargo.toml") -Raw
        if ($parentCargo -match 'govrix-scout') {
            $repoDir = Split-Path (Get-Location) -Parent
        }
    }

    if (-not $repoDir) {
        Write-Host ""
        Write-Fail @"
-Dev mode must be run from the cloned govrix-scout repo.

  Clone it first:
    git clone $REPO_URL
    cd govrix-scout
    .\install.ps1 -Dev
"@
    }

    Write-Info "Repo root: $repoDir"
    Set-Location $repoDir

    Test-Docker
    Test-OrInstallRust
    Test-Node
    Test-OrInstallPnpm

    # Dashboard dependencies
    Write-Info "Installing dashboard dependencies..."
    Set-Location (Join-Path $repoDir "dashboard")
    pnpm install --frozen-lockfile
    Set-Location $repoDir
    Write-Success "Dashboard dependencies installed"

    # Build Rust workspace
    Write-Info "Building Rust workspace (first build ~2-3 minutes)..."
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
    cargo build --workspace
    if ($LASTEXITCODE -ne 0) { Write-Fail "cargo build failed." }
    Write-Success "Rust workspace built"

    # Run tests
    Write-Info "Running tests..."
    $testOutput = cargo test --workspace 2>&1
    $testOutput | Where-Object { $_ -match '^test result|FAILED|error' } | Select-Object -First 20 | ForEach-Object { Write-Host $_ }
    Write-Success "Tests complete"

    # Start database
    Write-Info "Starting database (TimescaleDB)..."
    docker compose -f (Join-Path $repoDir "docker\docker-compose.yml") up -d postgres
    Wait-Healthy -Url "http://localhost:4001/health" -Label "database" -TimeoutSec 30 | Out-Null

    # Done
    Write-Host ""
    Write-Separator
    Write-Host "Dev environment ready!" -ForegroundColor Green
    Write-Host ""
    Write-Host "  make dev-proxy       # Start proxy in watch mode (ports 4000, 4001)"
    Write-Host "  make dev-dashboard   # Start React dashboard HMR (port 3000)"
    Write-Host "  make docker-up       # Start full stack in Docker"
    Write-Host "  make test            # Run all Rust tests"
    Write-Host "  make lint            # Run clippy"
    Write-Host "  make help            # All available commands"
    Write-Host ""
    Write-Host "  Proxy:     http://localhost:4000"
    Write-Host "  REST API:  http://localhost:4001"
    Write-Host "  Dashboard: http://localhost:3000"
    Write-Separator
    Write-Host ""
}

# =============================================================================
# Entrypoint
# =============================================================================
if ($Dev) {
    Install-Dev
} else {
    Install-User
}
