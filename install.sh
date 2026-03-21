#!/bin/sh
# Govrix Scout — AI Agent Governance
#
# One-line install:
#   curl -sSfL https://raw.githubusercontent.com/manaspros/govrix-scout/main/install.sh | sh
#
# Or clone and run:
#   ./install.sh
# ─────────────────────────────────────────────────────────────────────────────

set -eu

# ── Config ──────────────────────────────────────────────────────────────────
GOVRIX_DIR="$HOME/.govrix"
GOVRIX_BIN="$GOVRIX_DIR/bin"
COMPOSE_URL="https://raw.githubusercontent.com/manaspros/govrix-scout/main/docker/docker-compose.production.yml"
COMPOSE_FILE="$GOVRIX_DIR/docker-compose.yml"
HEALTH_URL="http://localhost:4001/health"
HEALTH_INTERVAL=2
HEALTH_TIMEOUT=60

# ── Colors (disabled if not a terminal) ─────────────────────────────────────
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    BOLD=''
    NC=''
fi

info()    { printf "%s\n" "$*"; }
success() { printf "${GREEN}[ok]${NC} %s\n" "$*"; }
warn()    { printf "${YELLOW}[warn]${NC} %s\n" "$*"; }
die()     { printf "${RED}[error]${NC} %s\n" "$*" >&2; exit 1; }

# ── Banner ──────────────────────────────────────────────────────────────────
banner() {
    printf "\n"
    printf "${BOLD}╔══════════════════════════════════════════════════════╗${NC}\n"
    printf "${BOLD}║     Govrix Scout — AI Agent Governance              ║${NC}\n"
    printf "${BOLD}╚══════════════════════════════════════════════════════╝${NC}\n"
    printf "\n"
}

# ── Check prerequisites ────────────────────────────────────────────────────
check_docker() {
    if ! command -v docker >/dev/null 2>&1; then
        die "Docker is not installed. Install it from https://docs.docker.com/get-docker/ and try again."
    fi

    if ! docker info >/dev/null 2>&1; then
        die "Docker daemon is not running. Start Docker Desktop and try again."
    fi

    if ! docker compose version >/dev/null 2>&1; then
        die "Docker Compose v2 is not available. Upgrade Docker or install the compose plugin: https://docs.docker.com/compose/install/"
    fi

    success "Docker and Docker Compose detected"
}

# ── Check port availability ─────────────────────────────────────────────────
check_port() {
    _port="$1"
    _label="$2"

    # Try multiple detection methods for POSIX compatibility
    if command -v ss >/dev/null 2>&1; then
        if ss -tlnp 2>/dev/null | grep -q ":${_port} "; then
            die "Port ${_port} (${_label}) is already in use. Free it and try again."
        fi
    elif command -v lsof >/dev/null 2>&1; then
        if lsof -iTCP:"${_port}" -sTCP:LISTEN >/dev/null 2>&1; then
            die "Port ${_port} (${_label}) is already in use. Free it and try again."
        fi
    elif command -v netstat >/dev/null 2>&1; then
        if netstat -tlnp 2>/dev/null | grep -q ":${_port} "; then
            die "Port ${_port} (${_label}) is already in use. Free it and try again."
        fi
    else
        warn "Cannot check if port ${_port} is in use (no ss/lsof/netstat). Continuing anyway."
    fi
}

check_ports() {
    check_port 3000 "Dashboard"
    check_port 4000 "Proxy"
    check_port 4001 "API"
    check_port 5432 "PostgreSQL"
    success "Required ports (3000, 4000, 4001, 5432) are available"
}

# ── Wait for health check ──────────────────────────────────────────────────
wait_for_health() {
    _elapsed=0
    printf "  Waiting for API health check "
    while [ "$_elapsed" -lt "$HEALTH_TIMEOUT" ]; do
        if curl -sSf "$HEALTH_URL" >/dev/null 2>&1; then
            printf "\n"
            success "API is healthy"
            return 0
        fi
        printf "."
        sleep "$HEALTH_INTERVAL"
        _elapsed=$(( _elapsed + HEALTH_INTERVAL ))
    done
    printf "\n"
    warn "API did not become healthy within ${HEALTH_TIMEOUT}s. Check logs with: govrix-scout logs"
    return 1
}

# ── Create CLI wrapper ──────────────────────────────────────────────────────
create_cli() {
    mkdir -p "$GOVRIX_BIN"

    cat > "$GOVRIX_BIN/govrix-scout" <<'WRAPPER'
#!/bin/sh
set -eu

GOVRIX_DIR="$HOME/.govrix"
COMPOSE_FILE="$GOVRIX_DIR/docker-compose.yml"

usage() {
    printf "Usage: govrix-scout <command>\n\n"
    printf "Commands:\n"
    printf "  start       Start Govrix Scout services\n"
    printf "  stop        Stop Govrix Scout services\n"
    printf "  logs        Follow service logs\n"
    printf "  status      Show service status and health\n"
    printf "  uninstall   Remove Govrix Scout completely\n"
    printf "\n"
}

case "${1:-}" in
    start)
        docker compose -f "$COMPOSE_FILE" up -d
        printf "Govrix Scout started.\n"
        printf "  Dashboard: http://localhost:3000\n"
        printf "  Proxy:     http://localhost:4000\n"
        printf "  API:       http://localhost:4001\n"
        ;;
    stop)
        docker compose -f "$COMPOSE_FILE" down
        printf "Govrix Scout stopped.\n"
        ;;
    logs)
        docker compose -f "$COMPOSE_FILE" logs -f
        ;;
    status)
        docker compose -f "$COMPOSE_FILE" ps
        printf "\nHealth check: "
        if curl -sSf http://localhost:4001/health 2>/dev/null; then
            printf "\n"
        else
            printf "UNREACHABLE\n"
        fi
        ;;
    uninstall)
        printf "This will stop all services and delete %s. Continue? [y/N] " "$GOVRIX_DIR"
        read -r confirm
        case "$confirm" in
            y|Y|yes|YES)
                docker compose -f "$COMPOSE_FILE" down -v 2>/dev/null || true
                rm -rf "$GOVRIX_DIR"
                printf "Govrix Scout uninstalled.\n"
                printf "You may want to remove ~/.govrix/bin from your PATH in ~/.bashrc or ~/.zshrc.\n"
                ;;
            *)
                printf "Aborted.\n"
                ;;
        esac
        ;;
    -h|--help|help|"")
        usage
        ;;
    *)
        printf "Unknown command: %s\n\n" "$1"
        usage
        exit 1
        ;;
esac
WRAPPER

    chmod +x "$GOVRIX_BIN/govrix-scout"
    success "CLI wrapper installed at $GOVRIX_BIN/govrix-scout"
}

# ── Add to PATH ─────────────────────────────────────────────────────────────
add_to_path() {
    # Check if already in PATH
    case ":$PATH:" in
        *":$GOVRIX_BIN:"*)
            return 0
            ;;
    esac

    _path_line="export PATH=\"\$HOME/.govrix/bin:\$PATH\""

    _added=0
    for _rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
        if [ -f "$_rc" ]; then
            if ! grep -qF '.govrix/bin' "$_rc" 2>/dev/null; then
                printf "\n# Govrix Scout\n%s\n" "$_path_line" >> "$_rc"
                _added=1
            fi
        fi
    done

    if [ "$_added" -eq 1 ]; then
        success "Added $GOVRIX_BIN to PATH (restart your shell or run: export PATH=\"\$HOME/.govrix/bin:\$PATH\")"
    else
        warn "Could not find .bashrc or .zshrc. Add this to your shell profile manually:"
        info "  $_path_line"
    fi

    # Make it available in the current session
    PATH="$GOVRIX_BIN:$PATH"
    export PATH
}

# ═══════════════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════════════

banner
check_docker
check_ports

# Create install directory
mkdir -p "$GOVRIX_DIR"
success "Created $GOVRIX_DIR"

# Download docker-compose.production.yml
info "Downloading docker-compose.yml..."
if command -v curl >/dev/null 2>&1; then
    curl -sSfL "$COMPOSE_URL" -o "$COMPOSE_FILE"
elif command -v wget >/dev/null 2>&1; then
    wget -qO "$COMPOSE_FILE" "$COMPOSE_URL"
else
    die "curl or wget is required. Install one and try again."
fi
success "Downloaded docker-compose.yml to $COMPOSE_FILE"

# Start services
info "Starting Govrix Scout services..."
docker compose -f "$COMPOSE_FILE" up -d
success "Services started"

# Wait for health
printf "\n"
wait_for_health || true

# Install CLI wrapper
printf "\n"
create_cli
add_to_path

# Print success
printf "\n"
printf "${BOLD}══════════════════════════════════════════════════════════${NC}\n"
printf "${GREEN}${BOLD}  Govrix Scout is running!${NC}\n"
printf "${BOLD}══════════════════════════════════════════════════════════${NC}\n"
printf "\n"
printf "  ${BOLD}Dashboard:${NC}  http://localhost:3000\n"
printf "  ${BOLD}Proxy:${NC}      http://localhost:4000\n"
printf "  ${BOLD}API:${NC}        http://localhost:4001\n"
printf "\n"
printf "  ${BOLD}Point your AI agents at Govrix (no code changes needed):${NC}\n"
printf "\n"
printf "    export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1\n"
printf "    export ANTHROPIC_BASE_URL=http://localhost:4000/proxy/anthropic/v1\n"
printf "\n"
printf "${BOLD}══════════════════════════════════════════════════════════${NC}\n"
printf "\n"
printf "  Run ${BOLD}govrix-scout status${NC} to check services\n"
printf "\n"
