#!/usr/bin/env bash
# Govrix — Installer
#
# Usage:
#   # End-user (Docker only, no Rust/Node needed):
#   curl -sSfL https://govrix.dev/install.sh | sh
#
#   # Contributor (full dev setup):
#   curl -sSfL https://govrix.dev/install.sh | sh -s -- --dev
#
#   # Or, after cloning:
#   ./install.sh          # Docker-only
#   ./install.sh --dev    # Full dev setup
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail

# ── Config ────────────────────────────────────────────────────────────────────
GOVRIX_DIR="${GOVRIX_DIR:-$HOME/.govrix}"
REPO_RAW_BASE="https://govrix.dev"
REPO_URL="https://github.com/Govrix-AI/govrix-scout"
MODE="user"

# ── Colors (disabled if not a terminal) ──────────────────────────────────────
if [ -t 1 ]; then
    RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
    BLUE='\033[0;34m'; BOLD='\033[1m'; NC='\033[0m'
else
    RED=''; GREEN=''; YELLOW=''; BLUE=''; BOLD=''; NC=''
fi

info()    { printf "${BLUE}[govrix]${NC} %s\n" "$*"; }
success() { printf "${GREEN}[govrix]${NC} %s\n" "$*"; }
warn()    { printf "${YELLOW}[govrix]${NC} WARN: %s\n" "$*"; }
error()   { printf "${RED}[govrix]${NC} ERROR: %s\n" "$*" >&2; exit 1; }

# ── Argument parsing ──────────────────────────────────────────────────────────
for arg in "$@"; do
    case "$arg" in
        --dev)   MODE="dev"  ;;
        --user)  MODE="user" ;;
        --help|-h)
            printf "Govrix Installer\n\n"
            printf "Usage: install.sh [--user|--dev]\n\n"
            printf "  (no flag)   Docker-only install. No Rust or Node.js needed.\n"
            printf "  --user      Same as above (explicit).\n"
            printf "  --dev       Full contributor setup. Requires Rust + Node.js 20+.\n\n"
            printf "Environment variables:\n"
            printf "  GOVRIX_DIR  Install directory (default: ~/.govrix)\n\n"
            exit 0
            ;;
        *) warn "Unknown argument: $arg (ignored)" ;;
    esac
done

# ── OS Detection ──────────────────────────────────────────────────────────────
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux"  ;;
        Darwin*) echo "macos"  ;;
        MINGW*|MSYS*|CYGWIN*)
            printf "\n"
            error "Windows native shell is not supported.\nPlease use WSL2: https://docs.microsoft.com/windows/wsl/install\nThen re-run this installer inside WSL2."
            ;;
        *)
            error "Unsupported OS: $(uname -s). Govrix supports Linux and macOS."
            ;;
    esac
}

# ── Download helper ───────────────────────────────────────────────────────────
download() {
    local url="$1" dest="$2"
    if command -v curl &>/dev/null; then
        curl -sSfL "$url" -o "$dest"
    elif command -v wget &>/dev/null; then
        wget -qO "$dest" "$url"
    else
        error "curl or wget is required. Install one and try again."
    fi
}

# ── Random hex (no openssl dependency) ───────────────────────────────────────
random_hex() {
    local len="${1:-32}"
    if [ -r /dev/urandom ]; then
        dd if=/dev/urandom bs=1 count="$len" 2>/dev/null \
            | od -An -tx1 \
            | tr -d ' \n' \
            | head -c "$len"
    else
        # Fallback: use date + process ID
        printf '%s%s' "$(date +%s)" "$$" | head -c "$len"
    fi
}

# ── Check Docker ──────────────────────────────────────────────────────────────
check_docker() {
    if ! command -v docker &>/dev/null; then
        printf "\n"
        error "Docker is not installed.\n\n  Install Docker Desktop from: https://docs.docker.com/get-docker/\n  Then re-run this installer.\n"
    fi

    if ! docker info &>/dev/null 2>&1; then
        printf "\n"
        error "Docker daemon is not running.\n\n  Start Docker Desktop, wait for it to be ready, then re-run this installer.\n"
    fi

    if ! docker compose version &>/dev/null 2>&1; then
        printf "\n"
        error "Docker Compose v2 is not available.\n\n  Upgrade Docker Desktop to include Compose v2.\n  See: https://docs.docker.com/compose/install/\n"
    fi

    local compose_ver
    compose_ver=$(docker compose version --short 2>/dev/null || echo "v2")
    success "Docker ready  (Compose $compose_ver)"
}

# ── Check + install Rust ──────────────────────────────────────────────────────
check_or_install_rust() {
    if command -v rustc &>/dev/null; then
        success "Rust found: $(rustc --version)"
        return
    fi
    info "Rust not found — installing via rustup (this takes ~2 minutes)..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    # Source for this shell session
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
    success "Rust installed: $(rustc --version)"
}

# ── Check Node.js ─────────────────────────────────────────────────────────────
check_node() {
    if ! command -v node &>/dev/null; then
        printf "\n"
        error "Node.js 20+ is required for --dev mode.\n\n  Install via nvm (recommended):\n    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash\n    nvm install 20 && nvm use 20\n\n  Or download from: https://nodejs.org\n"
    fi

    local node_major
    node_major=$(node --version | sed 's/v//' | cut -d. -f1)
    if [ "$node_major" -lt 20 ]; then
        error "Node.js 20+ required, found $(node --version).\n  Update: nvm install 20 && nvm use 20"
    fi
    success "Node.js $(node --version)"
}

# ── Check + install pnpm ──────────────────────────────────────────────────────
check_or_install_pnpm() {
    if command -v pnpm &>/dev/null; then
        success "pnpm $(pnpm --version)"
        return
    fi
    info "Installing pnpm..."
    npm install -g pnpm
    success "pnpm $(pnpm --version) installed"
}

# ── Wait for service health ───────────────────────────────────────────────────
wait_healthy() {
    local url="$1" label="$2" timeout="${3:-90}" interval=3 elapsed=0
    printf "${BLUE}[govrix]${NC} Waiting for %s" "$label"
    while [ "$elapsed" -lt "$timeout" ]; do
        if curl -sSf "$url" &>/dev/null; then
            printf " done\n"
            success "$label is ready"
            return 0
        fi
        printf "."
        sleep "$interval"
        elapsed=$((elapsed + interval))
    done
    printf "\n"
    warn "$label did not become ready in ${timeout}s — check logs: docker compose -C $GOVRIX_DIR logs"
    return 1
}

# ═════════════════════════════════════════════════════════════════════════════
# USER MODE — Docker-only, no Rust/Node needed
# ═════════════════════════════════════════════════════════════════════════════
install_user() {
    printf "\n${BOLD}Govrix — Quick Install (Docker)${NC}\n"
    printf "═══════════════════════════════════════════════════════════════\n\n"

    detect_os >/dev/null
    check_docker

    # Create install directory
    mkdir -p "$GOVRIX_DIR"
    info "Install directory: $GOVRIX_DIR"

    # Download docker-compose.yml
    local compose_dest="$GOVRIX_DIR/docker-compose.yml"
    if [ -f "$compose_dest" ]; then
        info "docker-compose.yml already exists — keeping"
    else
        info "Downloading docker-compose.yml..."
        download "$REPO_RAW_BASE/docker-compose.yml" "$compose_dest"
        success "docker-compose.yml downloaded"
    fi

    # Download database init SQL files
    local init_dir="$GOVRIX_DIR/init"
    mkdir -p "$init_dir"
    local sql_files="001_create_events.sql 002_create_agents.sql 003_create_costs.sql 004_create_hypertables.sql 005_create_indexes.sql 006_budget_daily.sql 007_budget_config.sql 008_create_projects.sql"
    local needs_sql=0
    for f in $sql_files; do
        [ ! -f "$init_dir/$f" ] && needs_sql=1 && break
    done
    if [ "$needs_sql" -eq 1 ]; then
        info "Downloading database migrations..."
        for f in $sql_files; do
            download "$REPO_RAW_BASE/init/$f" "$init_dir/$f"
        done
        success "Database migrations downloaded"
    else
        info "Database migrations already present — keeping"
    fi

    # Generate .env with secure random credentials
    local env_file="$GOVRIX_DIR/.env"
    if [ ! -f "$env_file" ]; then
        info "Generating .env with secure credentials..."
        local db_pass
        db_pass=$(random_hex 24)

        cat > "$env_file" <<EOF
# Govrix configuration — generated by install.sh $(date -u +"%Y-%m-%dT%H:%M:%SZ")
# Edit this file to change ports, credentials, or retention settings.

POSTGRES_USER=govrix
POSTGRES_DB=govrix
POSTGRES_PASSWORD=${db_pass}

GOVRIX_DATABASE__URL=postgres://govrix:${db_pass}@postgres:5432/govrix
GOVRIX_DATABASE__MAX_CONNECTIONS=20
GOVRIX_DATABASE__MIN_CONNECTIONS=2

GOVRIX_PROXY__LISTEN_ADDR=0.0.0.0:4000
GOVRIX_API__LISTEN_ADDR=0.0.0.0:4001
GOVRIX_METRICS__LISTEN_ADDR=0.0.0.0:9090

RUST_LOG=govrix_scout_proxy=info,tower_http=warn
EOF
        success ".env created"
    else
        success ".env already exists — keeping existing credentials"
    fi

    # Start services
    info "Pulling Docker images (first run takes 2-5 minutes)..."
    docker compose --project-directory "$GOVRIX_DIR" pull --quiet
    success "Images ready"

    info "Starting Govrix services..."
    docker compose --project-directory "$GOVRIX_DIR" up -d
    success "Services started"

    # Health checks
    printf "\n"
    wait_healthy "http://localhost:4001/health" "Govrix API"    90 || true
    wait_healthy "http://localhost:3000"         "Govrix Dashboard" 30 || true

    # Done
    printf "\n"
    printf "═══════════════════════════════════════════════════════════════\n"
    printf "${GREEN}${BOLD}Govrix is running!${NC}\n\n"
    printf "  ${BOLD}Dashboard:${NC}   http://localhost:3000\n"
    printf "  ${BOLD}API:${NC}         http://localhost:4001/health\n"
    printf "  ${BOLD}Metrics:${NC}     http://localhost:9090/metrics\n"
    printf "\n"
    printf "  ${BOLD}Point your agents at Govrix (one env var, no code changes):${NC}\n\n"
    printf "    export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1\n"
    printf "    export ANTHROPIC_BASE_URL=http://localhost:4000/proxy/anthropic/v1\n"
    printf "\n"
    printf "  ${BOLD}Manage:${NC}\n"
    printf "    docker compose --project-directory %s logs -f\n" "$GOVRIX_DIR"
    printf "    docker compose --project-directory %s down\n" "$GOVRIX_DIR"
    printf "    docker compose --project-directory %s up -d\n" "$GOVRIX_DIR"
    printf "\n"
    printf "  ${BOLD}Docs:${NC}  %s\n" "$REPO_URL"
    printf "═══════════════════════════════════════════════════════════════\n\n"
}

# ═════════════════════════════════════════════════════════════════════════════
# DEV MODE — Full contributor setup
# ═════════════════════════════════════════════════════════════════════════════
install_dev() {
    printf "\n${BOLD}Govrix — Developer Setup${NC}\n"
    printf "═══════════════════════════════════════════════════════════════\n\n"

    detect_os >/dev/null

    # Find repo root — must be run from the cloned repo
    local repo_dir
    if [ -f "Cargo.toml" ] && grep -q "govrix-scout" Cargo.toml 2>/dev/null; then
        repo_dir="$(pwd)"
    elif [ -f "../Cargo.toml" ] && grep -q "govrix-scout" ../Cargo.toml 2>/dev/null; then
        repo_dir="$(cd .. && pwd)"
    else
        printf "\n"
        error "--dev mode must be run from the cloned govrix-scout repo.\n\n  Clone it first:\n    git clone $REPO_URL\n    cd govrix-scout\n    ./install.sh --dev\n"
    fi
    info "Repo root: $repo_dir"
    cd "$repo_dir"

    check_docker
    check_or_install_rust
    check_node
    check_or_install_pnpm

    # Dashboard dependencies
    info "Installing dashboard dependencies..."
    cd "$repo_dir/dashboard"
    pnpm install --frozen-lockfile
    cd "$repo_dir"
    success "Dashboard dependencies installed"

    # Build Rust workspace
    info "Building Rust workspace (first build ~2-3 minutes)..."
    export PATH="$HOME/.cargo/bin:$PATH"
    cargo build --workspace
    success "Rust workspace built"

    # Run tests
    info "Running tests..."
    cargo test --workspace 2>&1 | grep -E "^test result|FAILED|error" | head -20
    success "Tests complete"

    # Start TimescaleDB
    info "Starting database (TimescaleDB)..."
    docker compose -f docker/docker-compose.yml up -d postgres
    wait_healthy "http://localhost:4001/health" "database" 30 || \
        info "Database starting — run 'make docker-up' when ready"

    # Done
    printf "\n"
    printf "═══════════════════════════════════════════════════════════════\n"
    printf "${GREEN}${BOLD}Dev environment ready!${NC}\n\n"
    printf "  make dev-proxy       # Start proxy in watch mode (ports 4000, 4001)\n"
    printf "  make dev-dashboard   # Start React dashboard HMR (port 3000)\n"
    printf "  make docker-up       # Start full stack in Docker\n"
    printf "  make test            # Run all Rust tests\n"
    printf "  make lint            # Run clippy\n"
    printf "  make help            # All available commands\n"
    printf "\n"
    printf "  Proxy:     http://localhost:4000\n"
    printf "  REST API:  http://localhost:4001\n"
    printf "  Dashboard: http://localhost:3000\n"
    printf "═══════════════════════════════════════════════════════════════\n\n"
}

# ─────────────────────────────────────────────────────────────────────────────
# Entrypoint
# ─────────────────────────────────────────────────────────────────────────────
case "$MODE" in
    user) install_user ;;
    dev)  install_dev  ;;
    *)    error "Unknown mode: $MODE" ;;
esac
