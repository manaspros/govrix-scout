#!/usr/bin/env bash
# Govrix Scout — Reset database (drop + recreate + re-migrate)
# ──────────────────────────────────────────────────────────────────────────────
# Usage: ./scripts/reset-db.sh [--yes]
#
# Drops the 'Govrix Scout' database inside the running postgres container,
# recreates it, and re-runs all migration files in order.
# Useful for starting completely fresh during development.
#
# Pass --yes (or -y) to skip the confirmation prompt.
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
success() { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
COMPOSE_FILE="$ROOT_DIR/docker/docker-compose.yml"
MIGRATIONS_DIR="$ROOT_DIR/init"

CONTAINER="govrix-scout-postgres"
PG_USER="Govrix Scout"
PG_DB="Govrix Scout"
PG_PASS="govrix_scout_dev"

echo ""
echo -e "${BOLD}${RED}Govrix Scout — Database Reset${NC}"
echo "────────────────────────────────────────────────────────────────"
warn "This will DROP and RECREATE the '$PG_DB' database."
warn "ALL data will be permanently deleted."
echo ""

# ── Confirmation ──────────────────────────────────────────────────────────────
SKIP_CONFIRM=false
for arg in "$@"; do
    if [[ "$arg" == "--yes" || "$arg" == "-y" ]]; then
        SKIP_CONFIRM=true
    fi
done

if [[ "$SKIP_CONFIRM" != "true" ]]; then
    read -r -p "Type 'reset' to confirm: " CONFIRM
    if [[ "$CONFIRM" != "reset" ]]; then
        info "Aborted — database was not modified."
        exit 0
    fi
fi

echo ""
info "Project root:    $ROOT_DIR"
info "Compose file:    $COMPOSE_FILE"
info "Migrations dir:  $MIGRATIONS_DIR"
echo ""

# ── Ensure postgres is running ────────────────────────────────────────────────
info "Checking postgres container..."

RUNNING=$(docker ps --filter "name=^${CONTAINER}$" --filter "status=running" -q 2>/dev/null || true)

if [[ -z "$RUNNING" ]]; then
    warn "Container '$CONTAINER' is not running. Starting postgres..."
    docker compose -f "$COMPOSE_FILE" up -d postgres

    info "Waiting for postgres to be healthy (up to 60s)..."
    MAX_WAIT=60
    WAITED=0
    until docker exec "$CONTAINER" \
            env PGPASSWORD="$PG_PASS" \
            pg_isready -U "$PG_USER" -d "postgres" -q 2>/dev/null; do
        sleep 2
        WAITED=$((WAITED + 2))
        if [[ $WAITED -ge $MAX_WAIT ]]; then
            error "Postgres did not become ready within ${MAX_WAIT}s."
        fi
        echo -n "."
    done
    echo ""
    success "Postgres is healthy"
else
    success "Postgres container is running"
fi

# ── Drop and recreate the database ───────────────────────────────────────────
info "Terminating existing connections to '$PG_DB'..."
docker exec -i "$CONTAINER" \
    env PGPASSWORD="$PG_PASS" \
    psql -U "$PG_USER" -d "postgres" -v ON_ERROR_STOP=1 -c "
SELECT pg_terminate_backend(pid)
FROM   pg_stat_activity
WHERE  datname = '$PG_DB'
AND    pid <> pg_backend_pid();
" > /dev/null 2>&1 || true

info "Dropping database '$PG_DB'..."
docker exec -i "$CONTAINER" \
    env PGPASSWORD="$PG_PASS" \
    psql -U "$PG_USER" -d "postgres" -v ON_ERROR_STOP=1 \
    -c "DROP DATABASE IF EXISTS $PG_DB;" 2>&1 | grep -v "^$" || true
success "Dropped '$PG_DB'"

info "Creating database '$PG_DB'..."
docker exec -i "$CONTAINER" \
    env PGPASSWORD="$PG_PASS" \
    psql -U "$PG_USER" -d "postgres" -v ON_ERROR_STOP=1 \
    -c "CREATE DATABASE $PG_DB OWNER $PG_USER;" 2>&1 | grep -v "^$" || true
success "Created '$PG_DB'"

echo ""

# ── Re-run migrations (dependency-aware order: 001,002,004,003,005) ──────────
# 004 must run before 003 because 003 needs TimescaleDB's time_bucket().
info "Running migrations..."

apply_mig() {
    local mig="$1"
    local fname
    fname="$(basename "$mig")"
    info "  Applying $fname ..."
    docker exec -i "$CONTAINER" \
        env PGPASSWORD="$PG_PASS" \
        psql -U "$PG_USER" -d "$PG_DB" -v ON_ERROR_STOP=1 \
        < "$mig" 2>&1 | grep -v "^$" | sed 's/^/    /' || true
    success "  $fname applied"
}

apply_mig "$MIGRATIONS_DIR/001_create_events.sql"
apply_mig "$MIGRATIONS_DIR/002_create_agents.sql"
apply_mig "$MIGRATIONS_DIR/004_create_hypertables.sql"
apply_mig "$MIGRATIONS_DIR/003_create_costs.sql"
apply_mig "$MIGRATIONS_DIR/005_create_indexes.sql"

echo ""
echo "────────────────────────────────────────────────────────────────"
echo -e "${GREEN}${BOLD}Database reset complete!${NC}"
echo ""
echo "The database is now empty with a fresh schema."
echo ""
echo "Next steps:"
echo "  Seed demo data:  ./scripts/seed-db.sh"
echo "────────────────────────────────────────────────────────────────"
