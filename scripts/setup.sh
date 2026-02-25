#!/usr/bin/env bash
# Govrix Scout — First-time setup script
# ──────────────────────────────────────────────────────────────────────────────
# Usage: ./scripts/setup.sh
#
# This script:
#   1. Checks required system dependencies
#   2. Installs Rust toolchain components (clippy, rustfmt)
#   3. Installs pnpm (if not present)
#   4. Installs dashboard node_modules
#   5. Runs cargo build to warm the cache
#   6. Prints next steps
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
success() { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

# ── Script directory ──────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

echo ""
echo -e "${BOLD}Govrix Scout OSS — First-time setup${NC}"
echo "────────────────────────────────────────────────────────────────"
info "Project root: $ROOT_DIR"
echo ""

# ── Check required dependencies ───────────────────────────────────────────────
info "Checking system dependencies..."

check_cmd() {
    local cmd="$1"
    local install_hint="$2"
    if ! command -v "$cmd" &>/dev/null; then
        error "$cmd is required but not installed. $install_hint"
    fi
    success "$cmd found: $(command -v "$cmd")"
}

# Rust (required)
check_cmd rustup "Install from https://rustup.rs"
check_cmd cargo  "Install from https://rustup.rs"

# Docker (required for the database)
check_cmd docker "Install from https://docs.docker.com/get-docker/"

if ! docker compose version &>/dev/null 2>&1; then
    error "Docker Compose v2 is required. Install from https://docs.docker.com/compose/"
fi
success "docker compose found"

# Git (required)
check_cmd git "Install git for your platform"

# ── Node.js check (optional but recommended) ──────────────────────────────────
NODE_AVAILABLE=false
if command -v node &>/dev/null; then
    NODE_VERSION=$(node --version)
    # Strip leading 'v' and compare major version
    NODE_MAJOR="${NODE_VERSION#v}"
    NODE_MAJOR="${NODE_MAJOR%%.*}"
    if [[ "$NODE_MAJOR" -lt 20 ]]; then
        warn "Node.js $NODE_VERSION found, but v20+ is required for dashboard development"
        warn "Install v20: https://nodejs.org or 'nvm install 20'"
    else
        success "node $NODE_VERSION found"
        NODE_AVAILABLE=true
    fi
else
    warn "Node.js not found — dashboard development will not be available"
    warn "Install from https://nodejs.org or use 'nvm install 20'"
fi

# ── Rust toolchain ────────────────────────────────────────────────────────────
echo ""
info "Setting up Rust toolchain..."

rustup update stable 2>&1 | grep -E "^(info|unchanged|updated)" || true
rustup component add clippy rustfmt 2>&1 | grep -E "^(info|Downloading|Installing|Installed|already)" || true

RUST_VERSION=$(rustc --version)
success "Rust: $RUST_VERSION"

# ── pnpm + dashboard ──────────────────────────────────────────────────────────
if [[ "$NODE_AVAILABLE" == "true" ]]; then
    echo ""
    info "Setting up dashboard dependencies..."

    if ! command -v pnpm &>/dev/null; then
        info "Installing pnpm via corepack..."
        corepack enable
        corepack prepare pnpm@latest --activate
    fi

    PNPM_VERSION=$(pnpm --version)
    success "pnpm $PNPM_VERSION"

    info "Installing dashboard node_modules..."
    cd "$ROOT_DIR/dashboard"
    pnpm install --frozen-lockfile
    cd "$ROOT_DIR"
    success "Dashboard dependencies installed"
fi

# ── Build the Rust workspace ──────────────────────────────────────────────────
echo ""
info "Building Rust workspace (this may take a few minutes on first run)..."
cd "$ROOT_DIR"

if cargo build --workspace 2>&1 | tail -3 | grep -qE "^error"; then
    error "cargo build failed. See output above."
fi

success "Rust workspace built successfully"

# ── Run tests ─────────────────────────────────────────────────────────────────
info "Running unit tests..."
TEST_OUTPUT=$(cargo test --workspace --lib --bins 2>&1)
TEST_SUMMARY=$(echo "$TEST_OUTPUT" | grep -E "^test result" | tail -1)
success "Tests: $TEST_SUMMARY"

# ── Environment file ──────────────────────────────────────────────────────────
if [[ -f "$ROOT_DIR/.env.example" ]] && [[ ! -f "$ROOT_DIR/.env" ]]; then
    cp "$ROOT_DIR/.env.example" "$ROOT_DIR/.env"
    success "Created .env from .env.example"
fi

# ── Final summary ─────────────────────────────────────────────────────────────
echo ""
echo "────────────────────────────────────────────────────────────────"
echo -e "${GREEN}${BOLD}Setup complete!${NC}"
echo ""
echo "Next steps:"
echo ""
echo -e "  ${BOLD}1. Start the full stack:${NC}"
echo "       docker compose -f docker/docker-compose.yml up -d"
echo ""
echo -e "  ${BOLD}2. Open the dashboard:${NC}"
echo "       http://localhost:3000"
echo ""
echo -e "  ${BOLD}3. Verify the proxy:${NC}"
echo "       curl http://localhost:4001/health"
echo ""
echo -e "  ${BOLD}4. Point an agent at the proxy:${NC}"
echo "       export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1"
echo ""
echo -e "  ${BOLD}5. Seed demo data (optional):${NC}"
echo "       ./scripts/seed-demo-data.sh"
echo ""
echo "  Proxy:     http://localhost:4000"
echo "  REST API:  http://localhost:4001"
echo "  Dashboard: http://localhost:3000"
echo "  Metrics:   http://localhost:9090/metrics"
echo "────────────────────────────────────────────────────────────────"
