#!/usr/bin/env bash
# Govrix Scout — Direct DB seed script (inserts realistic demo data into TimescaleDB)
# ──────────────────────────────────────────────────────────────────────────────
# Usage: ./scripts/seed-db.sh
#
# 1. Ensures postgres container is running and healthy
# 2. Runs all migration SQL files (001-005) in order
# 3. Inserts 5 agents and ~100 events spread over the last 7 days
# 4. Refreshes the cost_daily materialized view
# 5. Prints row counts and dashboard URL
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

# ── Colours ───────────────────────────────────────────────────────────────────
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

# ── Paths ─────────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
COMPOSE_FILE="$ROOT_DIR/docker/docker-compose.yml"
MIGRATIONS_DIR="$ROOT_DIR/migrations"

# ── DB connection settings (match docker-compose.yml) ─────────────────────────
CONTAINER="govrix-scout-postgres"
PG_USER="Govrix Scout"
PG_DB="Govrix Scout"
PG_PASS="govrix_scout_dev"

echo ""
echo -e "${BOLD}Govrix Scout — Database Seed (Direct Insert)${NC}"
echo "────────────────────────────────────────────────────────────────"
info "Project root:    $ROOT_DIR"
info "Compose file:    $COMPOSE_FILE"
info "Migrations dir:  $MIGRATIONS_DIR"
echo ""

# ── Helper: run SQL via docker exec ──────────────────────────────────────────
psql_exec() {
    docker exec -i "$CONTAINER" \
        env PGPASSWORD="$PG_PASS" \
        psql -U "$PG_USER" -d "$PG_DB" -v ON_ERROR_STOP=1 "$@"
}

psql_query() {
    docker exec -i "$CONTAINER" \
        env PGPASSWORD="$PG_PASS" \
        psql -U "$PG_USER" -d "$PG_DB" -tA -c "$1"
}

# ── Step 1: Check / start postgres ───────────────────────────────────────────
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
            pg_isready -U "$PG_USER" -d "$PG_DB" -q 2>/dev/null; do
        sleep 2
        WAITED=$((WAITED + 2))
        if [[ $WAITED -ge $MAX_WAIT ]]; then
            error "Postgres did not become ready within ${MAX_WAIT}s. Check: docker logs $CONTAINER"
        fi
        echo -n "."
    done
    echo ""
    success "Postgres is healthy"
else
    if ! docker exec "$CONTAINER" \
            env PGPASSWORD="$PG_PASS" \
            pg_isready -U "$PG_USER" -d "$PG_DB" -q 2>/dev/null; then
        error "Container '$CONTAINER' is running but postgres is not accepting connections yet."
    fi
    success "Postgres container is running and accepting connections"
fi

echo ""

# ── Step 2: Run migrations in dependency order ────────────────────────────────
# 004 (hypertables) must run before 003 (cost_daily view) because 003 needs
# TimescaleDB's time_bucket() function which is installed by 004.
# Correct order: 001, 002, 004, 003, 005
info "Running migrations (dependency-aware order: 001,002,004,003,005)..."

apply_migration() {
    local mig="$1"
    local fname
    fname="$(basename "$mig")"
    info "  Applying $fname ..."
    psql_exec < "$mig" 2>&1 | grep -v "^$" | sed 's/^/    /' || true
    success "  $fname applied"
}

apply_migration "$MIGRATIONS_DIR/001_create_events.sql"
apply_migration "$MIGRATIONS_DIR/002_create_agents.sql"
apply_migration "$MIGRATIONS_DIR/004_create_hypertables.sql"
apply_migration "$MIGRATIONS_DIR/003_create_costs.sql"
apply_migration "$MIGRATIONS_DIR/005_create_indexes.sql"

echo ""

# ── Step 3: Insert demo data ──────────────────────────────────────────────────
info "Inserting demo agents and events..."

psql_exec << 'EOSQL'

-- ─────────────────────────────────────────────────────────────────────────
-- AGENTS (5 rows)
-- ─────────────────────────────────────────────────────────────────────────
INSERT INTO agents (
    id, name, description, agent_type, status,
    source_ip, fingerprint,
    target_apis, mcp_servers,
    total_requests, total_tokens_in, total_tokens_out, total_cost_usd,
    last_model_used, error_count,
    labels, metadata,
    first_seen_at, last_seen_at,
    created_at, updated_at
) VALUES

(
    'research-bot-001',
    'Research Bot',
    'Automated research assistant that summarises papers, fetches web data, and produces structured reports.',
    'langchain', 'active',
    '10.0.1.11'::inet, 'a3f8c2e1d9b7465f3c2a1e8d7b6c5f4a',
    '["https://api.openai.com/v1"]'::jsonb, '[]'::jsonb,
    12847, 58234190, 21456780, 142.30000000,
    'gpt-4o', 23,
    '{"team": "research", "env": "production", "owner": "ml-team"}'::jsonb,
    '{"version": "1.4.2", "framework_version": "0.2.1"}'::jsonb,
    NOW() - INTERVAL '8 days', NOW() - INTERVAL '2 minutes',
    NOW() - INTERVAL '8 days', NOW() - INTERVAL '2 minutes'
),

(
    'code-assistant-042',
    'Code Assistant',
    'Multi-step code review, generation, and refactoring agent built with CrewAI.',
    'crewai', 'active',
    '10.0.1.42'::inet, 'b7c1d3e5f9a2468b4d6e8f0c2a4b6d8e',
    '["https://api.openai.com/v1"]'::jsonb, '[]'::jsonb,
    8231, 31109230, 12874010, 89.44000000,
    'gpt-4o-mini', 11,
    '{"team": "engineering", "env": "production", "squad": "platform"}'::jsonb,
    '{"version": "2.1.0", "framework_version": "0.6.3"}'::jsonb,
    NOW() - INTERVAL '7 days', NOW() - INTERVAL '15 minutes',
    NOW() - INTERVAL '7 days', NOW() - INTERVAL '15 minutes'
),

(
    'support-agent-007',
    'Support Agent',
    'Customer-facing support triage agent. Classifies tickets and drafts first-response emails.',
    'autogen', 'active',
    '10.0.2.7'::inet, 'c9e2f4a6b8d0472c6e8a0c2e4f6a8b0c',
    '["https://api.anthropic.com/v1"]'::jsonb, '[]'::jsonb,
    342, 1283640, 502180, 3.71000000,
    'claude-haiku-4-5-20251001', 4,
    '{"team": "support", "env": "production", "tier": "starter"}'::jsonb,
    '{"version": "0.3.1", "framework_version": "0.2.0"}'::jsonb,
    NOW() - INTERVAL '6 days', NOW() - INTERVAL '1 hour',
    NOW() - INTERVAL '6 days', NOW() - INTERVAL '1 hour'
),

(
    'data-pipeline-019',
    'Data Pipeline',
    'Nightly ETL pipeline agent that processes raw data, calls LLMs for enrichment, and writes to the data warehouse.',
    'custom', 'idle',
    '10.0.3.19'::inet, 'd5f7a9c1e3b5479d7f9b1d3f5a7c9e1b',
    '["https://api.anthropic.com/v1", "https://api.openai.com/v1"]'::jsonb,
    '["mcp://filesystem", "mcp://database"]'::jsonb,
    1205, 9876540, 3210980, 67.89000000,
    'claude-sonnet-4-20250514', 7,
    '{"team": "data", "env": "production", "schedule": "nightly"}'::jsonb,
    '{"version": "1.0.0", "pipeline": "etl-v3"}'::jsonb,
    NOW() - INTERVAL '7 days', NOW() - INTERVAL '6 hours',
    NOW() - INTERVAL '7 days', NOW() - INTERVAL '6 hours'
),

(
    'unknown-10.0.3.7',
    NULL, NULL,
    'unknown', 'active',
    '10.0.3.7'::inet, 'e1a3c5e7f9b1453e1a3c5e7f9b1453e1',
    '["https://api.openai.com/v1"]'::jsonb, '[]'::jsonb,
    17, 34210, 14890, 0.89000000,
    'gpt-4o-mini', 2,
    '{"env": "unknown"}'::jsonb, '{}'::jsonb,
    NOW() - INTERVAL '3 days', NOW() - INTERVAL '4 hours',
    NOW() - INTERVAL '3 days', NOW() - INTERVAL '4 hours'
)

ON CONFLICT (id) DO UPDATE SET
    total_requests   = EXCLUDED.total_requests,
    total_tokens_in  = EXCLUDED.total_tokens_in,
    total_tokens_out = EXCLUDED.total_tokens_out,
    total_cost_usd   = EXCLUDED.total_cost_usd,
    last_seen_at     = EXCLUDED.last_seen_at,
    updated_at       = NOW();

-- ─────────────────────────────────────────────────────────────────────────
-- EVENTS (~100 rows over 7 days)
-- ─────────────────────────────────────────────────────────────────────────

INSERT INTO events (
    id, session_id, agent_id,
    timestamp, latency_ms,
    direction, method, upstream_target, provider, model,
    status_code, finish_reason,
    payload, raw_size_bytes,
    input_tokens, output_tokens, total_tokens, cost_usd,
    pii_detected, tools_called, lineage_hash, compliance_tag,
    tags, error_message, created_at
) VALUES

-- ═══════════ DAY 7 — today (~18 events) ═══════════

-- Session A: research-bot-001 | gpt-4o | 4 turns
(gen_random_uuid(), 'a1000001-0000-4000-8000-000000000001', 'research-bot-001',
 NOW() - INTERVAL '1 hour 45 minutes', 1243, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":3,"has_tools":true,"tool_names":["web_search","cite_paper"],"temperature":0.3}'::jsonb,
 4821, 2150, NULL, 2150, 0.02150000,
 '[]'::jsonb, '["web_search","cite_paper"]'::jsonb,
 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 hour 45 minutes'),

(gen_random_uuid(), 'a1000001-0000-4000-8000-000000000001', 'research-bot-001',
 NOW() - INTERVAL '1 hour 43 minutes', 3817, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":2150,"completion_tokens":892}}'::jsonb,
 9234, 2150, 892, 3042, 0.03042000,
 '[]'::jsonb, '[]'::jsonb,
 'b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 hour 43 minutes'),

(gen_random_uuid(), 'a1000001-0000-4000-8000-000000000001', 'research-bot-001',
 NOW() - INTERVAL '1 hour 40 minutes', 1089, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":5,"has_tools":true,"tool_names":["web_search","calculator"],"temperature":0.3}'::jsonb,
 5102, 3100, NULL, 3100, 0.03100000,
 '[]'::jsonb, '["web_search","calculator"]'::jsonb,
 'c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 hour 40 minutes'),

(gen_random_uuid(), 'a1000001-0000-4000-8000-000000000001', 'research-bot-001',
 NOW() - INTERVAL '1 hour 37 minutes', 4201, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":true,"tool_calls":[{"name":"calculator"}],"usage":{"prompt_tokens":3100,"completion_tokens":1240}}'::jsonb,
 11872, 3100, 1240, 4340, 0.04340000,
 '[]'::jsonb, '["calculator"]'::jsonb,
 'd4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 hour 37 minutes'),

-- Session B: code-assistant-042 | gpt-4o-mini
(gen_random_uuid(), 'b1000002-0000-4000-8000-000000000002', 'code-assistant-042',
 NOW() - INTERVAL '2 hours 10 minutes', 678, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, NULL,
 '{"model":"gpt-4o-mini","messages_count":2,"has_tools":false,"temperature":0.1}'::jsonb,
 1843, 890, NULL, 890, 0.00089000,
 '[]'::jsonb, '[]'::jsonb,
 'e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '2 hours 10 minutes'),

(gen_random_uuid(), 'b1000002-0000-4000-8000-000000000002', 'code-assistant-042',
 NOW() - INTERVAL '2 hours 9 minutes', 1542, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, 'stop',
 '{"model":"gpt-4o-mini","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":890,"completion_tokens":412}}'::jsonb,
 3891, 890, 412, 1302, 0.00130200,
 '[]'::jsonb, '[]'::jsonb,
 'f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '2 hours 9 minutes'),

-- Session C: research-bot-001 | gpt-4o | PII email warning
(gen_random_uuid(), 'c1000003-0000-4000-8000-000000000003', 'research-bot-001',
 NOW() - INTERVAL '3 hours 5 minutes', 1105, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":1,"has_tools":false,"temperature":0.7}'::jsonb,
 2341, 1420, NULL, 1420, 0.01420000,
 '[{"pii_type":"EMAIL_ADDRESS","location":"messages[0].content","confidence":0.97}]'::jsonb, '[]'::jsonb,
 'a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8',
 'warn:pii_email', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '3 hours 5 minutes'),

(gen_random_uuid(), 'c1000003-0000-4000-8000-000000000003', 'research-bot-001',
 NOW() - INTERVAL '3 hours 3 minutes', 2887, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":1420,"completion_tokens":634}}'::jsonb,
 6123, 1420, 634, 2054, 0.02054000,
 '[]'::jsonb, '[]'::jsonb,
 'b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9',
 'warn:pii_email', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '3 hours 3 minutes'),

-- Session D: support-agent-007 | claude-haiku | anthropic
(gen_random_uuid(), 'd1000004-0000-4000-8000-000000000004', 'support-agent-007',
 NOW() - INTERVAL '1 hour 20 minutes', 543, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-haiku-4-5-20251001',
 200, NULL,
 '{"model":"claude-haiku-4-5-20251001","messages_count":2,"has_tools":false,"temperature":0.5}'::jsonb,
 1209, 520, NULL, 520, 0.00052000,
 '[]'::jsonb, '[]'::jsonb,
 'c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0',
 'pass:all', '{"team":"support","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 hour 20 minutes'),

(gen_random_uuid(), 'd1000004-0000-4000-8000-000000000004', 'support-agent-007',
 NOW() - INTERVAL '1 hour 18 minutes', 1897, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-haiku-4-5-20251001',
 200, 'end_turn',
 '{"model":"claude-haiku-4-5-20251001","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":520,"output_tokens":213}}'::jsonb,
 2341, 520, 213, 733, 0.00073300,
 '[]'::jsonb, '[]'::jsonb,
 'd0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1',
 'pass:all', '{"team":"support","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 hour 18 minutes'),

-- Session E: code-assistant-042 | gpt-4o | large context, cost_budget warn
(gen_random_uuid(), 'e1000005-0000-4000-8000-000000000005', 'code-assistant-042',
 NOW() - INTERVAL '30 minutes', 892, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":8,"has_tools":true,"tool_names":["code_interpreter","github_search"],"temperature":0.0}'::jsonb,
 7812, 6200, NULL, 6200, 0.06200000,
 '[]'::jsonb, '["code_interpreter","github_search"]'::jsonb,
 'e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2',
 'warn:cost_budget', '{"team":"engineering","env":"production","squad":"platform"}'::jsonb, NULL, NOW() - INTERVAL '30 minutes'),

(gen_random_uuid(), 'e1000005-0000-4000-8000-000000000005', 'code-assistant-042',
 NOW() - INTERVAL '27 minutes', 6341, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":true,"tool_calls":[{"name":"code_interpreter"},{"name":"github_search"}],"usage":{"prompt_tokens":6200,"completion_tokens":3812}}'::jsonb,
 42891, 6200, 3812, 10012, 0.10012000,
 '[]'::jsonb, '["code_interpreter","github_search"]'::jsonb,
 'f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7',
 'warn:cost_budget', '{"team":"engineering","env":"production","squad":"platform"}'::jsonb, NULL, NOW() - INTERVAL '27 minutes'),

-- Session F: research-bot-001 | 429 rate-limit error
(gen_random_uuid(), 'f1000006-0000-4000-8000-000000000006', 'research-bot-001',
 NOW() - INTERVAL '45 minutes', 312, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 429, NULL,
 '{"model":"gpt-4o","messages_count":4,"has_tools":true,"tool_names":["web_search"],"temperature":0.3}'::jsonb,
 2198, 1800, NULL, 1800, 0.00000000,
 '[]'::jsonb, '["web_search"]'::jsonb,
 'a7c8d9e0f1a2a7c8d9e0f1a2a7c8d9e0f1a2a7c8d9e0f1a2a7c8d9e0f1a2a7c8',
 'warn:upstream_error', '{"team":"research","env":"production"}'::jsonb,
 'Rate limit exceeded: 429 Too Many Requests from api.openai.com', NOW() - INTERVAL '45 minutes'),

-- Session G: data-pipeline-019 | mcp + claude-sonnet
(gen_random_uuid(), '07000007-0000-4000-8000-000000000007', 'data-pipeline-019',
 NOW() - INTERVAL '6 hours 30 minutes', 234, 'outbound', 'POST',
 'mcp://filesystem/read', 'mcp', NULL,
 200, NULL,
 '{"mcp_method":"tools/call","tool_name":"read_file","arguments":{"path":"/data/raw/batch_2024_02.csv"}}'::jsonb,
 892, NULL, NULL, NULL, 0.00000000,
 '[]'::jsonb, '["read_file"]'::jsonb,
 'b8d9e0f1a2b8d9e0f1a2b8d9e0f1a2b8d9e0f1a2b8d9e0f1a2b8d9e0f1a2b8d9',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '6 hours 30 minutes'),

(gen_random_uuid(), '07000007-0000-4000-8000-000000000007', 'data-pipeline-019',
 NOW() - INTERVAL '6 hours 28 minutes', 4312, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, NULL,
 '{"model":"claude-sonnet-4-20250514","messages_count":1,"has_tools":false,"temperature":0.0}'::jsonb,
 8932, 7800, NULL, 7800, 0.07800000,
 '[]'::jsonb, '[]'::jsonb,
 'c9e0f1a2b8c9e0f1a2b8c9e0f1a2b8c9e0f1a2b8c9e0f1a2b8c9e0f1a2b8c9e0',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '6 hours 28 minutes'),

(gen_random_uuid(), '07000007-0000-4000-8000-000000000007', 'data-pipeline-019',
 NOW() - INTERVAL '6 hours 24 minutes', 5891, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, 'end_turn',
 '{"model":"claude-sonnet-4-20250514","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":7800,"completion_tokens":2341}}'::jsonb,
 24892, 7800, 2341, 10141, 0.10141000,
 '[]'::jsonb, '[]'::jsonb,
 'd0f1a2b8c9d0f1a2b8c9d0f1a2b8c9d0f1a2b8c9d0f1a2b8c9d0f1a2b8c9d0f1',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '6 hours 24 minutes'),

-- Session H: unknown-10.0.3.7 | gpt-4o-mini
(gen_random_uuid(), '08000008-0000-4000-8000-000000000008', 'unknown-10.0.3.7',
 NOW() - INTERVAL '4 hours', 421, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, NULL,
 '{"model":"gpt-4o-mini","messages_count":1,"has_tools":false,"temperature":1.0}'::jsonb,
 987, 410, NULL, 410, 0.00041000,
 '[]'::jsonb, '[]'::jsonb,
 'e1a2b8c9d0e1a2b8c9d0e1a2b8c9d0e1a2b8c9d0e1a2b8c9d0e1a2b8c9d0e1a2',
 'pass:all', '{"env":"unknown"}'::jsonb, NULL, NOW() - INTERVAL '4 hours'),

(gen_random_uuid(), '08000008-0000-4000-8000-000000000008', 'unknown-10.0.3.7',
 NOW() - INTERVAL '3 hours 58 minutes', 1234, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, 'stop',
 '{"model":"gpt-4o-mini","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":410,"completion_tokens":187}}'::jsonb,
 1789, 410, 187, 597, 0.00059700,
 '[]'::jsonb, '[]'::jsonb,
 'f2a2b8c9d0e1f2a2b8c9d0e1f2a2b8c9d0e1f2a2b8c9d0e1f2a2b8c9d0e1f2a2',
 'pass:all', '{"env":"unknown"}'::jsonb, NULL, NOW() - INTERVAL '3 hours 58 minutes'),

-- ═══════════ DAY 6 (~14 events) ═══════════

(gen_random_uuid(), '09000009-0000-4000-8000-000000000009', 'research-bot-001',
 NOW() - INTERVAL '1 day 3 hours', 1432, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":2,"has_tools":true,"tool_names":["web_search"],"temperature":0.3}'::jsonb,
 3421, 1780, NULL, 1780, 0.01780000,
 '[]'::jsonb, '["web_search"]'::jsonb,
 'a1c2d3e4f5a1c2d3e4f5a1c2d3e4f5a1c2d3e4f5a1c2d3e4f5a1c2d3e4f5a1c2',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 day 3 hours'),

(gen_random_uuid(), '09000009-0000-4000-8000-000000000009', 'research-bot-001',
 NOW() - INTERVAL '1 day 2 hours 57 minutes', 3102, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":1780,"completion_tokens":743}}'::jsonb,
 8012, 1780, 743, 2523, 0.02523000,
 '[]'::jsonb, '[]'::jsonb,
 'b2d3e4f5a1b2d3e4f5a1b2d3e4f5a1b2d3e4f5a1b2d3e4f5a1b2d3e4f5a1b2d3',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 day 2 hours 57 minutes'),

(gen_random_uuid(), '10000010-0000-4000-8000-000000000010', 'code-assistant-042',
 NOW() - INTERVAL '1 day 5 hours', 521, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, NULL,
 '{"model":"gpt-4o-mini","messages_count":3,"has_tools":false,"temperature":0.2}'::jsonb,
 2341, 1102, NULL, 1102, 0.00110200,
 '[]'::jsonb, '[]'::jsonb,
 'c3e4f5a1b2c3e4f5a1b2c3e4f5a1b2c3e4f5a1b2c3e4f5a1b2c3e4f5a1b2c3e4',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 day 5 hours'),

(gen_random_uuid(), '10000010-0000-4000-8000-000000000010', 'code-assistant-042',
 NOW() - INTERVAL '1 day 4 hours 58 minutes', 2341, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, 'stop',
 '{"model":"gpt-4o-mini","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":1102,"completion_tokens":498}}'::jsonb,
 4812, 1102, 498, 1600, 0.00160000,
 '[]'::jsonb, '[]'::jsonb,
 'd4f5a1b2c3d4f5a1b2c3d4f5a1b2c3d4f5a1b2c3d4f5a1b2c3d4f5a1b2c3d4f5',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 day 4 hours 58 minutes'),

-- PII phone warning
(gen_random_uuid(), '11000011-0000-4000-8000-000000000011', 'research-bot-001',
 NOW() - INTERVAL '1 day 8 hours', 987, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":1,"has_tools":false,"temperature":0.7}'::jsonb,
 1892, 1340, NULL, 1340, 0.01340000,
 '[{"pii_type":"PHONE_NUMBER","location":"messages[0].content","confidence":0.94},{"pii_type":"PERSON","location":"messages[0].content","confidence":0.89}]'::jsonb, '[]'::jsonb,
 'e5a1b2c3d4e5a1b2c3d4e5a1b2c3d4e5a1b2c3d4e5a1b2c3d4e5a1b2c3d4e5a1',
 'warn:pii_phone', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 day 8 hours'),

(gen_random_uuid(), '11000011-0000-4000-8000-000000000011', 'research-bot-001',
 NOW() - INTERVAL '1 day 7 hours 57 minutes', 2102, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":1340,"completion_tokens":512}}'::jsonb,
 5481, 1340, 512, 1852, 0.01852000,
 '[]'::jsonb, '[]'::jsonb,
 'f6b2c3d4e5f6b2c3d4e5f6b2c3d4e5f6b2c3d4e5f6b2c3d4e5f6b2c3d4e5f6b2',
 'warn:pii_phone', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 day 7 hours 57 minutes'),

(gen_random_uuid(), '12000012-0000-4000-8000-000000000012', 'support-agent-007',
 NOW() - INTERVAL '1 day 2 hours', 412, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-haiku-4-5-20251001',
 200, NULL,
 '{"model":"claude-haiku-4-5-20251001","messages_count":1,"has_tools":false,"temperature":0.5}'::jsonb,
 891, 430, NULL, 430, 0.00043000,
 '[]'::jsonb, '[]'::jsonb,
 'a7b1c2d3e4a7b1c2d3e4a7b1c2d3e4a7b1c2d3e4a7b1c2d3e4a7b1c2d3e4a7b1',
 'pass:all', '{"team":"support","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 day 2 hours'),

(gen_random_uuid(), '12000012-0000-4000-8000-000000000012', 'support-agent-007',
 NOW() - INTERVAL '1 day 1 hour 58 minutes', 1543, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-haiku-4-5-20251001',
 200, 'end_turn',
 '{"model":"claude-haiku-4-5-20251001","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":430,"output_tokens":189}}'::jsonb,
 2102, 430, 189, 619, 0.00061900,
 '[]'::jsonb, '[]'::jsonb,
 'b8c2d3e4a7b8c2d3e4a7b8c2d3e4a7b8c2d3e4a7b8c2d3e4a7b8c2d3e4a7b8c2',
 'pass:all', '{"team":"support","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '1 day 1 hour 58 minutes'),

(gen_random_uuid(), '13000013-0000-4000-8000-000000000013', 'data-pipeline-019',
 NOW() - INTERVAL '1 day 6 hours', 198, 'outbound', 'POST',
 'mcp://database/query', 'mcp', NULL,
 200, NULL,
 '{"mcp_method":"tools/call","tool_name":"query_db","arguments":{"sql":"SELECT * FROM raw_events WHERE date = current_date - 1"}}'::jsonb,
 1102, NULL, NULL, NULL, 0.00000000,
 '[]'::jsonb, '["query_db"]'::jsonb,
 'c9d3e4a7b8c9d3e4a7b8c9d3e4a7b8c9d3e4a7b8c9d3e4a7b8c9d3e4a7b8c9d3',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '1 day 6 hours'),

(gen_random_uuid(), '13000013-0000-4000-8000-000000000013', 'data-pipeline-019',
 NOW() - INTERVAL '1 day 5 hours 55 minutes', 3892, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, NULL,
 '{"model":"claude-sonnet-4-20250514","messages_count":2,"has_tools":false,"temperature":0.0}'::jsonb,
 6781, 5400, NULL, 5400, 0.05400000,
 '[]'::jsonb, '[]'::jsonb,
 'd0e4a7b8c9d0e4a7b8c9d0e4a7b8c9d0e4a7b8c9d0e4a7b8c9d0e4a7b8c9d0e4',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '1 day 5 hours 55 minutes'),

(gen_random_uuid(), '13000013-0000-4000-8000-000000000013', 'data-pipeline-019',
 NOW() - INTERVAL '1 day 5 hours 49 minutes', 7234, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, 'end_turn',
 '{"model":"claude-sonnet-4-20250514","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":5400,"completion_tokens":1892}}'::jsonb,
 19234, 5400, 1892, 7292, 0.07292000,
 '[]'::jsonb, '[]'::jsonb,
 'e1f4a7b8c9d0e1f4a7b8c9d0e1f4a7b8c9d0e1f4a7b8c9d0e1f4a7b8c9d0e1f4',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '1 day 5 hours 49 minutes'),

(gen_random_uuid(), '14000014-0000-4000-8000-000000000014', 'unknown-10.0.3.7',
 NOW() - INTERVAL '1 day 4 hours', 389, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, NULL,
 '{"model":"gpt-4o-mini","messages_count":1,"has_tools":false,"temperature":1.0}'::jsonb,
 812, 380, NULL, 380, 0.00038000,
 '[]'::jsonb, '[]'::jsonb,
 'f2a4b7c8d9e0f2a4b7c8d9e0f2a4b7c8d9e0f2a4b7c8d9e0f2a4b7c8d9e0f2a4',
 'pass:all', '{"env":"unknown"}'::jsonb, NULL, NOW() - INTERVAL '1 day 4 hours'),

(gen_random_uuid(), '14000014-0000-4000-8000-000000000014', 'unknown-10.0.3.7',
 NOW() - INTERVAL '1 day 3 hours 58 minutes', 1102, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, 'stop',
 '{"model":"gpt-4o-mini","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":380,"completion_tokens":162}}'::jsonb,
 1564, 380, 162, 542, 0.00054200,
 '[]'::jsonb, '[]'::jsonb,
 'a3b4c5d6e7f8a3b4c5d6e7f8a3b4c5d6e7f8a3b4c5d6e7f8a3b4c5d6e7f8a3b4',
 'pass:all', '{"env":"unknown"}'::jsonb, NULL, NOW() - INTERVAL '1 day 3 hours 58 minutes'),

-- ═══════════ DAY 5 (~14 events) ═══════════

(gen_random_uuid(), '15000015-0000-4000-8000-000000000015', 'research-bot-001',
 NOW() - INTERVAL '2 days 4 hours', 1892, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":4,"has_tools":true,"tool_names":["web_search","cite_paper","summarize"],"temperature":0.3}'::jsonb,
 5231, 3200, NULL, 3200, 0.03200000,
 '[]'::jsonb, '["web_search","cite_paper","summarize"]'::jsonb,
 'b4c5d6e7f8a1b4c5d6e7f8a1b4c5d6e7f8a1b4c5d6e7f8a1b4c5d6e7f8a1b4c5',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '2 days 4 hours'),

(gen_random_uuid(), '15000015-0000-4000-8000-000000000015', 'research-bot-001',
 NOW() - INTERVAL '2 days 3 hours 56 minutes', 4891, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":true,"tool_calls":[{"name":"web_search"},{"name":"summarize"}],"usage":{"prompt_tokens":3200,"completion_tokens":1876}}'::jsonb,
 21023, 3200, 1876, 5076, 0.05076000,
 '[]'::jsonb, '["web_search","summarize"]'::jsonb,
 'c5d6e7f8a1b2c5d6e7f8a1b2c5d6e7f8a1b2c5d6e7f8a1b2c5d6e7f8a1b2c5d6',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '2 days 3 hours 56 minutes'),

(gen_random_uuid(), '15000015-0000-4000-8000-000000000015', 'research-bot-001',
 NOW() - INTERVAL '2 days 3 hours 50 minutes', 1234, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":6,"has_tools":true,"tool_names":["cite_paper"],"temperature":0.3}'::jsonb,
 4102, 2890, NULL, 2890, 0.02890000,
 '[]'::jsonb, '["cite_paper"]'::jsonb,
 'd6e7f8a1b2c3d6e7f8a1b2c3d6e7f8a1b2c3d6e7f8a1b2c3d6e7f8a1b2c3d6e7',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '2 days 3 hours 50 minutes'),

(gen_random_uuid(), '15000015-0000-4000-8000-000000000015', 'research-bot-001',
 NOW() - INTERVAL '2 days 3 hours 44 minutes', 3102, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":2890,"completion_tokens":1023}}'::jsonb,
 11234, 2890, 1023, 3913, 0.03913000,
 '[]'::jsonb, '[]'::jsonb,
 'e7f8a1b2c3d4e7f8a1b2c3d4e7f8a1b2c3d4e7f8a1b2c3d4e7f8a1b2c3d4e7f8',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '2 days 3 hours 44 minutes'),

(gen_random_uuid(), '16000016-0000-4000-8000-000000000016', 'code-assistant-042',
 NOW() - INTERVAL '2 days 7 hours', 734, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":5,"has_tools":true,"tool_names":["code_interpreter"],"temperature":0.0}'::jsonb,
 4812, 3780, NULL, 3780, 0.03780000,
 '[]'::jsonb, '["code_interpreter"]'::jsonb,
 'f8a1b2c3d4e5f8a1b2c3d4e5f8a1b2c3d4e5f8a1b2c3d4e5f8a1b2c3d4e5f8a1',
 'pass:all', '{"team":"engineering","env":"production","squad":"platform"}'::jsonb, NULL, NOW() - INTERVAL '2 days 7 hours'),

(gen_random_uuid(), '16000016-0000-4000-8000-000000000016', 'code-assistant-042',
 NOW() - INTERVAL '2 days 6 hours 55 minutes', 5234, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":true,"tool_calls":[{"name":"code_interpreter"}],"usage":{"prompt_tokens":3780,"completion_tokens":2134}}'::jsonb,
 24891, 3780, 2134, 5914, 0.05914000,
 '[]'::jsonb, '["code_interpreter"]'::jsonb,
 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
 'warn:cost_budget', '{"team":"engineering","env":"production","squad":"platform"}'::jsonb, NULL, NOW() - INTERVAL '2 days 6 hours 55 minutes'),

-- 500 error
(gen_random_uuid(), '17000017-0000-4000-8000-000000000017', 'support-agent-007',
 NOW() - INTERVAL '2 days 3 hours', 289, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-haiku-4-5-20251001',
 500, NULL,
 '{"model":"claude-haiku-4-5-20251001","messages_count":3,"has_tools":false,"temperature":0.5}'::jsonb,
 1893, 640, NULL, 640, 0.00000000,
 '[]'::jsonb, '[]'::jsonb,
 'b2c3d4e5f6a7b2c3d4e5f6a7b2c3d4e5f6a7b2c3d4e5f6a7b2c3d4e5f6a7b2c3',
 'warn:upstream_error', '{"team":"support","env":"production"}'::jsonb,
 'Internal server error: 500 from api.anthropic.com — retrying in 5s', NOW() - INTERVAL '2 days 3 hours'),

(gen_random_uuid(), '18000018-0000-4000-8000-000000000018', 'data-pipeline-019',
 NOW() - INTERVAL '2 days 5 hours 30 minutes', 4102, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, NULL,
 '{"model":"claude-sonnet-4-20250514","messages_count":1,"has_tools":false,"temperature":0.0}'::jsonb,
 7231, 6800, NULL, 6800, 0.06800000,
 '[]'::jsonb, '[]'::jsonb,
 'c3d4e5f6a7b8c3d4e5f6a7b8c3d4e5f6a7b8c3d4e5f6a7b8c3d4e5f6a7b8c3d4',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '2 days 5 hours 30 minutes'),

(gen_random_uuid(), '18000018-0000-4000-8000-000000000018', 'data-pipeline-019',
 NOW() - INTERVAL '2 days 5 hours 23 minutes', 6912, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, 'end_turn',
 '{"model":"claude-sonnet-4-20250514","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":6800,"completion_tokens":2134}}'::jsonb,
 22891, 6800, 2134, 8934, 0.08934000,
 '[]'::jsonb, '[]'::jsonb,
 'd4e5f6a7b8c9d4e5f6a7b8c9d4e5f6a7b8c9d4e5f6a7b8c9d4e5f6a7b8c9d4e5',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '2 days 5 hours 23 minutes'),

-- ═══════════ DAY 4 (~12 events) ═══════════

(gen_random_uuid(), '19000019-0000-4000-8000-000000000019', 'research-bot-001',
 NOW() - INTERVAL '3 days 5 hours', 1102, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":3,"has_tools":true,"tool_names":["web_search"],"temperature":0.3}'::jsonb,
 3812, 2100, NULL, 2100, 0.02100000,
 '[]'::jsonb, '["web_search"]'::jsonb,
 'e5f6a7b8c9d0e5f6a7b8c9d0e5f6a7b8c9d0e5f6a7b8c9d0e5f6a7b8c9d0e5f6',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '3 days 5 hours'),

(gen_random_uuid(), '19000019-0000-4000-8000-000000000019', 'research-bot-001',
 NOW() - INTERVAL '3 days 4 hours 56 minutes', 3212, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":2100,"completion_tokens":891}}'::jsonb,
 9234, 2100, 891, 2991, 0.02991000,
 '[]'::jsonb, '[]'::jsonb,
 'f6a7b8c9d0e1f6a7b8c9d0e1f6a7b8c9d0e1f6a7b8c9d0e1f6a7b8c9d0e1f6a7',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '3 days 4 hours 56 minutes'),

(gen_random_uuid(), '20000020-0000-4000-8000-000000000020', 'code-assistant-042',
 NOW() - INTERVAL '3 days 8 hours', 498, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, NULL,
 '{"model":"gpt-4o-mini","messages_count":2,"has_tools":false,"temperature":0.1}'::jsonb,
 1892, 780, NULL, 780, 0.00078000,
 '[]'::jsonb, '[]'::jsonb,
 'a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8c9d0e1f2a7b8',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '3 days 8 hours'),

(gen_random_uuid(), '20000020-0000-4000-8000-000000000020', 'code-assistant-042',
 NOW() - INTERVAL '3 days 7 hours 58 minutes', 1812, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, 'stop',
 '{"model":"gpt-4o-mini","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":780,"completion_tokens":342}}'::jsonb,
 3412, 780, 342, 1122, 0.00112200,
 '[]'::jsonb, '[]'::jsonb,
 'b8c9d0e1f2a3b8c9d0e1f2a3b8c9d0e1f2a3b8c9d0e1f2a3b8c9d0e1f2a3b8c9',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '3 days 7 hours 58 minutes'),

(gen_random_uuid(), '21000021-0000-4000-8000-000000000021', 'data-pipeline-019',
 NOW() - INTERVAL '3 days 5 hours 30 minutes', 215, 'outbound', 'POST',
 'mcp://filesystem/write', 'mcp', NULL,
 200, NULL,
 '{"mcp_method":"tools/call","tool_name":"write_file","arguments":{"path":"/data/processed/output_2024_02.json"}}'::jsonb,
 1231, NULL, NULL, NULL, 0.00000000,
 '[]'::jsonb, '["write_file"]'::jsonb,
 'c9d0e1f2a3b4c9d0e1f2a3b4c9d0e1f2a3b4c9d0e1f2a3b4c9d0e1f2a3b4c9d0',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '3 days 5 hours 30 minutes'),

(gen_random_uuid(), '21000021-0000-4000-8000-000000000021', 'data-pipeline-019',
 NOW() - INTERVAL '3 days 5 hours 25 minutes', 3812, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, NULL,
 '{"model":"claude-sonnet-4-20250514","messages_count":1,"has_tools":false,"temperature":0.0}'::jsonb,
 5892, 4900, NULL, 4900, 0.04900000,
 '[]'::jsonb, '[]'::jsonb,
 'd0e1f2a3b4c5d0e1f2a3b4c5d0e1f2a3b4c5d0e1f2a3b4c5d0e1f2a3b4c5d0e1',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '3 days 5 hours 25 minutes'),

(gen_random_uuid(), '21000021-0000-4000-8000-000000000021', 'data-pipeline-019',
 NOW() - INTERVAL '3 days 5 hours 18 minutes', 5921, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, 'end_turn',
 '{"model":"claude-sonnet-4-20250514","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":4900,"completion_tokens":1782}}'::jsonb,
 18234, 4900, 1782, 6682, 0.06682000,
 '[]'::jsonb, '[]'::jsonb,
 'e1f2a3b4c5d6e1f2a3b4c5d6e1f2a3b4c5d6e1f2a3b4c5d6e1f2a3b4c5d6e1f2',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '3 days 5 hours 18 minutes'),

(gen_random_uuid(), '22000022-0000-4000-8000-000000000022', 'unknown-10.0.3.7',
 NOW() - INTERVAL '3 days 2 hours', 512, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, NULL,
 '{"model":"gpt-4o-mini","messages_count":1,"has_tools":false,"temperature":1.0}'::jsonb,
 934, 520, NULL, 520, 0.00052000,
 '[]'::jsonb, '[]'::jsonb,
 'f2a3b4c5d6e7f2a3b4c5d6e7f2a3b4c5d6e7f2a3b4c5d6e7f2a3b4c5d6e7f2a3',
 'pass:all', '{"env":"unknown"}'::jsonb, NULL, NOW() - INTERVAL '3 days 2 hours'),

(gen_random_uuid(), '22000022-0000-4000-8000-000000000022', 'unknown-10.0.3.7',
 NOW() - INTERVAL '3 days 1 hour 58 minutes', 987, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, 'stop',
 '{"model":"gpt-4o-mini","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":520,"completion_tokens":231}}'::jsonb,
 2123, 520, 231, 751, 0.00075100,
 '[]'::jsonb, '[]'::jsonb,
 'a3b4c5d6e7f8a3b4c5d6e7f8a3b4c5d6e7f8a3b4c5d6e7f8a3b4c5d6e7f8a3b4',
 'pass:all', '{"env":"unknown"}'::jsonb, NULL, NOW() - INTERVAL '3 days 1 hour 58 minutes'),

-- ═══════════ DAY 3 (~10 events) ═══════════

(gen_random_uuid(), '23000023-0000-4000-8000-000000000023', 'research-bot-001',
 NOW() - INTERVAL '4 days 6 hours', 1432, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":2,"has_tools":true,"tool_names":["web_search","cite_paper"],"temperature":0.3}'::jsonb,
 3812, 1920, NULL, 1920, 0.01920000,
 '[]'::jsonb, '["web_search","cite_paper"]'::jsonb,
 'b4c5d6e7f8a9b4c5d6e7f8a9b4c5d6e7f8a9b4c5d6e7f8a9b4c5d6e7f8a9b4c5',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '4 days 6 hours'),

(gen_random_uuid(), '23000023-0000-4000-8000-000000000023', 'research-bot-001',
 NOW() - INTERVAL '4 days 5 hours 56 minutes', 2891, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":1920,"completion_tokens":712}}'::jsonb,
 7812, 1920, 712, 2632, 0.02632000,
 '[]'::jsonb, '[]'::jsonb,
 'c5d6e7f8a9b0c5d6e7f8a9b0c5d6e7f8a9b0c5d6e7f8a9b0c5d6e7f8a9b0c5d6',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '4 days 5 hours 56 minutes'),

(gen_random_uuid(), '24000024-0000-4000-8000-000000000024', 'code-assistant-042',
 NOW() - INTERVAL '4 days 4 hours', 892, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":4,"has_tools":true,"tool_names":["code_interpreter","github_search"],"temperature":0.0}'::jsonb,
 5231, 4100, NULL, 4100, 0.04100000,
 '[]'::jsonb, '["code_interpreter","github_search"]'::jsonb,
 'd6e7f8a9b0c1d6e7f8a9b0c1d6e7f8a9b0c1d6e7f8a9b0c1d6e7f8a9b0c1d6e7',
 'pass:all', '{"team":"engineering","env":"production","squad":"platform"}'::jsonb, NULL, NOW() - INTERVAL '4 days 4 hours'),

(gen_random_uuid(), '24000024-0000-4000-8000-000000000024', 'code-assistant-042',
 NOW() - INTERVAL '4 days 3 hours 53 minutes', 5102, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":true,"tool_calls":[{"name":"code_interpreter"}],"usage":{"prompt_tokens":4100,"completion_tokens":2234}}'::jsonb,
 25891, 4100, 2234, 6334, 0.06334000,
 '[]'::jsonb, '["code_interpreter"]'::jsonb,
 'e7f8a9b0c1d2e7f8a9b0c1d2e7f8a9b0c1d2e7f8a9b0c1d2e7f8a9b0c1d2e7f8',
 'warn:cost_budget', '{"team":"engineering","env":"production","squad":"platform"}'::jsonb, NULL, NOW() - INTERVAL '4 days 3 hours 53 minutes'),

(gen_random_uuid(), '25000025-0000-4000-8000-000000000025', 'support-agent-007',
 NOW() - INTERVAL '4 days 2 hours', 398, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-haiku-4-5-20251001',
 200, NULL,
 '{"model":"claude-haiku-4-5-20251001","messages_count":2,"has_tools":false,"temperature":0.5}'::jsonb,
 1023, 490, NULL, 490, 0.00049000,
 '[]'::jsonb, '[]'::jsonb,
 'f8a9b0c1d2e3f8a9b0c1d2e3f8a9b0c1d2e3f8a9b0c1d2e3f8a9b0c1d2e3f8a9',
 'pass:all', '{"team":"support","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '4 days 2 hours'),

(gen_random_uuid(), '25000025-0000-4000-8000-000000000025', 'support-agent-007',
 NOW() - INTERVAL '4 days 1 hour 58 minutes', 1234, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-haiku-4-5-20251001',
 200, 'end_turn',
 '{"model":"claude-haiku-4-5-20251001","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":490,"output_tokens":198}}'::jsonb,
 2102, 490, 198, 688, 0.00068800,
 '[]'::jsonb, '[]'::jsonb,
 'a9b0c1d2e3f4a9b0c1d2e3f4a9b0c1d2e3f4a9b0c1d2e3f4a9b0c1d2e3f4a9b0',
 'pass:all', '{"team":"support","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '4 days 1 hour 58 minutes'),

(gen_random_uuid(), '26000026-0000-4000-8000-000000000026', 'data-pipeline-019',
 NOW() - INTERVAL '4 days 5 hours 30 minutes', 3812, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, NULL,
 '{"model":"claude-sonnet-4-20250514","messages_count":1,"has_tools":false,"temperature":0.0}'::jsonb,
 6712, 5600, NULL, 5600, 0.05600000,
 '[]'::jsonb, '[]'::jsonb,
 'b0c1d2e3f4a5b0c1d2e3f4a5b0c1d2e3f4a5b0c1d2e3f4a5b0c1d2e3f4a5b0c1',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '4 days 5 hours 30 minutes'),

(gen_random_uuid(), '26000026-0000-4000-8000-000000000026', 'data-pipeline-019',
 NOW() - INTERVAL '4 days 5 hours 22 minutes', 6102, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, 'end_turn',
 '{"model":"claude-sonnet-4-20250514","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":5600,"completion_tokens":1912}}'::jsonb,
 19834, 5600, 1912, 7512, 0.07512000,
 '[]'::jsonb, '[]'::jsonb,
 'c1d2e3f4a5b6c1d2e3f4a5b6c1d2e3f4a5b6c1d2e3f4a5b6c1d2e3f4a5b6c1d2',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '4 days 5 hours 22 minutes'),

-- ═══════════ DAY 2 (~8 events) ═══════════

(gen_random_uuid(), 'aa100027-0000-4000-8000-000000000027', 'research-bot-001',
 NOW() - INTERVAL '5 days 7 hours', 1782, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":3,"has_tools":true,"tool_names":["web_search"],"temperature":0.3}'::jsonb,
 3291, 2340, NULL, 2340, 0.02340000,
 '[]'::jsonb, '["web_search"]'::jsonb,
 'd2e3f4a5b6c7d2e3f4a5b6c7d2e3f4a5b6c7d2e3f4a5b6c7d2e3f4a5b6c7d2e3',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '5 days 7 hours'),

(gen_random_uuid(), 'aa100027-0000-4000-8000-000000000027', 'research-bot-001',
 NOW() - INTERVAL '5 days 6 hours 56 minutes', 3102, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":2340,"completion_tokens":934}}'::jsonb,
 9812, 2340, 934, 3274, 0.03274000,
 '[]'::jsonb, '[]'::jsonb,
 'e3f4a5b6c7d8e3f4a5b6c7d8e3f4a5b6c7d8e3f4a5b6c7d8e3f4a5b6c7d8e3f4',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '5 days 6 hours 56 minutes'),

(gen_random_uuid(), 'ab100028-0000-4000-8000-000000000028', 'code-assistant-042',
 NOW() - INTERVAL '5 days 4 hours', 612, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, NULL,
 '{"model":"gpt-4o-mini","messages_count":2,"has_tools":false,"temperature":0.1}'::jsonb,
 2102, 1010, NULL, 1010, 0.00101000,
 '[]'::jsonb, '[]'::jsonb,
 'f4a5b6c7d8e9f4a5b6c7d8e9f4a5b6c7d8e9f4a5b6c7d8e9f4a5b6c7d8e9f4a5',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '5 days 4 hours'),

(gen_random_uuid(), 'ab100028-0000-4000-8000-000000000028', 'code-assistant-042',
 NOW() - INTERVAL '5 days 3 hours 58 minutes', 1892, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o-mini',
 200, 'stop',
 '{"model":"gpt-4o-mini","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":1010,"completion_tokens":456}}'::jsonb,
 4891, 1010, 456, 1466, 0.00146600,
 '[]'::jsonb, '[]'::jsonb,
 'a5b6c7d8e9f0a5b6c7d8e9f0a5b6c7d8e9f0a5b6c7d8e9f0a5b6c7d8e9f0a5b6',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '5 days 3 hours 58 minutes'),

(gen_random_uuid(), 'ac100029-0000-4000-8000-000000000029', 'data-pipeline-019',
 NOW() - INTERVAL '5 days 5 hours 30 minutes', 4102, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, NULL,
 '{"model":"claude-sonnet-4-20250514","messages_count":1,"has_tools":false,"temperature":0.0}'::jsonb,
 6231, 5200, NULL, 5200, 0.05200000,
 '[]'::jsonb, '[]'::jsonb,
 'b6c7d8e9f0a1b6c7d8e9f0a1b6c7d8e9f0a1b6c7d8e9f0a1b6c7d8e9f0a1b6c7',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '5 days 5 hours 30 minutes'),

(gen_random_uuid(), 'ac100029-0000-4000-8000-000000000029', 'data-pipeline-019',
 NOW() - INTERVAL '5 days 5 hours 23 minutes', 6234, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, 'end_turn',
 '{"model":"claude-sonnet-4-20250514","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":5200,"completion_tokens":1634}}'::jsonb,
 17812, 5200, 1634, 6834, 0.06834000,
 '[]'::jsonb, '[]'::jsonb,
 'c7d8e9f0a1b2c7d8e9f0a1b2c7d8e9f0a1b2c7d8e9f0a1b2c7d8e9f0a1b2c7d8',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '5 days 5 hours 23 minutes'),

(gen_random_uuid(), 'ad100030-0000-4000-8000-000000000030', 'support-agent-007',
 NOW() - INTERVAL '5 days 3 hours', 412, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-haiku-4-5-20251001',
 200, NULL,
 '{"model":"claude-haiku-4-5-20251001","messages_count":1,"has_tools":false,"temperature":0.5}'::jsonb,
 934, 410, NULL, 410, 0.00041000,
 '[]'::jsonb, '[]'::jsonb,
 'd8e9f0a1b2c3d8e9f0a1b2c3d8e9f0a1b2c3d8e9f0a1b2c3d8e9f0a1b2c3d8e9',
 'pass:all', '{"team":"support","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '5 days 3 hours'),

(gen_random_uuid(), 'ad100030-0000-4000-8000-000000000030', 'support-agent-007',
 NOW() - INTERVAL '5 days 2 hours 58 minutes', 1432, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-haiku-4-5-20251001',
 200, 'end_turn',
 '{"model":"claude-haiku-4-5-20251001","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":410,"output_tokens":178}}'::jsonb,
 1934, 410, 178, 588, 0.00058800,
 '[]'::jsonb, '[]'::jsonb,
 'e9f0a1b2c3d4e9f0a1b2c3d4e9f0a1b2c3d4e9f0a1b2c3d4e9f0a1b2c3d4e9f0',
 'pass:all', '{"team":"support","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '5 days 2 hours 58 minutes'),

-- ═══════════ DAY 1 — oldest (~8 events) ═══════════

(gen_random_uuid(), 'ae100031-0000-4000-8000-000000000031', 'research-bot-001',
 NOW() - INTERVAL '6 days 8 hours', 2102, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":2,"has_tools":false,"temperature":0.7}'::jsonb,
 2891, 1560, NULL, 1560, 0.01560000,
 '[]'::jsonb, '[]'::jsonb,
 'f0a1b2c3d4e5f0a1b2c3d4e5f0a1b2c3d4e5f0a1b2c3d4e5f0a1b2c3d4e5f0a1',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '6 days 8 hours'),

(gen_random_uuid(), 'ae100031-0000-4000-8000-000000000031', 'research-bot-001',
 NOW() - INTERVAL '6 days 7 hours 56 minutes', 2891, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":false,"usage":{"prompt_tokens":1560,"completion_tokens":689}}'::jsonb,
 7123, 1560, 689, 2249, 0.02249000,
 '[]'::jsonb, '[]'::jsonb,
 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
 'pass:all', '{"team":"research","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '6 days 7 hours 56 minutes'),

(gen_random_uuid(), 'af100032-0000-4000-8000-000000000032', 'code-assistant-042',
 NOW() - INTERVAL '6 days 5 hours', 734, 'outbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, NULL,
 '{"model":"gpt-4o","messages_count":3,"has_tools":true,"tool_names":["code_interpreter"],"temperature":0.0}'::jsonb,
 3812, 2780, NULL, 2780, 0.02780000,
 '[]'::jsonb, '["code_interpreter"]'::jsonb,
 'b2c3d4e5f6a7b2c3d4e5f6a7b2c3d4e5f6a7b2c3d4e5f6a7b2c3d4e5f6a7b2c3',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '6 days 5 hours'),

(gen_random_uuid(), 'af100032-0000-4000-8000-000000000032', 'code-assistant-042',
 NOW() - INTERVAL '6 days 4 hours 54 minutes', 4891, 'inbound', 'POST',
 'https://api.openai.com/v1/chat/completions', 'openai', 'gpt-4o',
 200, 'stop',
 '{"model":"gpt-4o","finish_reason":"stop","choices_count":1,"has_tool_calls":true,"tool_calls":[{"name":"code_interpreter"}],"usage":{"prompt_tokens":2780,"completion_tokens":1543}}'::jsonb,
 18923, 2780, 1543, 4323, 0.04323000,
 '[]'::jsonb, '["code_interpreter"]'::jsonb,
 'c3d4e5f6a7b8c3d4e5f6a7b8c3d4e5f6a7b8c3d4e5f6a7b8c3d4e5f6a7b8c3d4',
 'pass:all', '{"team":"engineering","env":"production"}'::jsonb, NULL, NOW() - INTERVAL '6 days 4 hours 54 minutes'),

(gen_random_uuid(), '33000033-0000-4000-8000-000000000033', 'data-pipeline-019',
 NOW() - INTERVAL '6 days 5 hours 30 minutes', 3891, 'outbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, NULL,
 '{"model":"claude-sonnet-4-20250514","messages_count":1,"has_tools":false,"temperature":0.0}'::jsonb,
 5812, 4800, NULL, 4800, 0.04800000,
 '[]'::jsonb, '[]'::jsonb,
 'd4e5f6a7b8c9d4e5f6a7b8c9d4e5f6a7b8c9d4e5f6a7b8c9d4e5f6a7b8c9d4e5',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '6 days 5 hours 30 minutes'),

(gen_random_uuid(), '33000033-0000-4000-8000-000000000033', 'data-pipeline-019',
 NOW() - INTERVAL '6 days 5 hours 22 minutes', 5812, 'inbound', 'POST',
 'https://api.anthropic.com/v1/messages', 'anthropic', 'claude-sonnet-4-20250514',
 200, 'end_turn',
 '{"model":"claude-sonnet-4-20250514","finish_reason":"end_turn","choices_count":1,"has_tool_calls":false,"usage":{"input_tokens":4800,"completion_tokens":1712}}'::jsonb,
 18234, 4800, 1712, 6512, 0.06512000,
 '[]'::jsonb, '[]'::jsonb,
 'e5f6a7b8c9d0e5f6a7b8c9d0e5f6a7b8c9d0e5f6a7b8c9d0e5f6a7b8c9d0e5f6',
 'pass:all', '{"team":"data","env":"production","schedule":"nightly"}'::jsonb, NULL, NOW() - INTERVAL '6 days 5 hours 22 minutes');

-- Refresh the materialized view
REFRESH MATERIALIZED VIEW cost_daily;

EOSQL

echo ""
success "Demo data inserted successfully"

# ── Step 4: Verify row counts ─────────────────────────────────────────────────
echo ""
info "Verifying data..."

EVENT_COUNT=$(psql_query "SELECT COUNT(*) FROM events;")
AGENT_COUNT=$(psql_query "SELECT COUNT(*) FROM agents;")
SESSION_COUNT=$(psql_query "SELECT COUNT(DISTINCT session_id) FROM events;")
ERROR_COUNT=$(psql_query "SELECT COUNT(*) FROM events WHERE status_code NOT IN (200) AND status_code IS NOT NULL;")
PII_COUNT=$(psql_query "SELECT COUNT(*) FROM events WHERE pii_detected != '[]';")
COST_DAILY_COUNT=$(psql_query "SELECT COUNT(*) FROM cost_daily;")
TOTAL_COST=$(psql_query "SELECT COALESCE(ROUND(SUM(cost_usd)::numeric, 4), 0) FROM events;")

echo ""
echo "────────────────────────────────────────────────────────────────"
echo -e "${BOLD}Seed Summary${NC}"
echo "────────────────────────────────────────────────────────────────"
echo -e "  Events inserted:        ${GREEN}${EVENT_COUNT}${NC}"
echo -e "  Agents registered:      ${GREEN}${AGENT_COUNT}${NC}"
echo -e "  Distinct sessions:      ${GREEN}${SESSION_COUNT}${NC}"
echo -e "  Error events (4xx/5xx): ${YELLOW}${ERROR_COUNT}${NC}"
echo -e "  PII warning events:     ${YELLOW}${PII_COUNT}${NC}"
echo -e "  cost_daily rows:        ${GREEN}${COST_DAILY_COUNT}${NC}"
echo -e "  Total cost (USD):       ${GREEN}\$${TOTAL_COST}${NC}"
echo "────────────────────────────────────────────────────────────────"

# ── Step 5: Per-agent event breakdown ─────────────────────────────────────────
echo ""
info "Per-agent event breakdown:"
echo ""
psql_exec -c "
SELECT
    a.id            AS agent,
    a.agent_type    AS framework,
    a.status,
    COUNT(e.id)     AS events,
    ROUND(COALESCE(SUM(e.cost_usd), 0)::numeric, 4) AS cost_usd
FROM agents a
LEFT JOIN events e ON e.agent_id = a.id
GROUP BY a.id, a.agent_type, a.status
ORDER BY events DESC;
" 2>/dev/null || true

echo ""
echo "────────────────────────────────────────────────────────────────"
echo -e "${GREEN}${BOLD}Seed complete!${NC}"
echo ""
echo "  Dashboard:  http://localhost:3000"
echo "  REST API:   http://localhost:4001/api/v1/events"
echo "  Agents API: http://localhost:4001/api/v1/agents"
echo "  Costs API:  http://localhost:4001/api/v1/costs/summary"
echo ""
echo "  Quick verify:"
echo "    docker exec govrix-scout-postgres env PGPASSWORD=govrix_scout_dev \\"
echo "      psql -U Govrix Scout -d Govrix Scout -c 'SELECT COUNT(*) FROM events;'"
echo "────────────────────────────────────────────────────────────────"
