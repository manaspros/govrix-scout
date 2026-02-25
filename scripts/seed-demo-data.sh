#!/usr/bin/env bash
# Govrix Scout — Seed demo data
# ──────────────────────────────────────────────────────────────────────────────
# Usage: ./scripts/seed-demo-data.sh [PROXY_URL]
#
# Sends five synthetic OpenAI-format requests through the Govrix Scout proxy.
# The proxy forwards them to the configured upstream (which will return a 401
# or connection error since we use a fake API key). The proxy still captures
# and stores the request events — so the dashboard will show real activity.
#
# Each request uses a different agent identity to demonstrate multi-agent
# tracking in the dashboard.
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

PROXY_URL="${1:-http://localhost:4000}"
API_URL="${PROXY_URL%:4000}:4001"
OPENAI_ENDPOINT="$PROXY_URL/proxy/openai/v1/chat/completions"
ANTHROPIC_ENDPOINT="$PROXY_URL/proxy/anthropic/v1/messages"

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { echo -e "${BLUE}[INFO]${NC}  $*"; }
success() { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC}  $*"; }
error()   { echo -e "${RED}[ERROR]${NC} $*"; }

echo ""
echo "Govrix Scout — Seeding demo data"
echo "────────────────────────────────────────────────────────────────"
echo "Proxy:   $PROXY_URL"
echo "API:     $API_URL"
echo ""

# ── Check proxy is running ────────────────────────────────────────────────────
info "Checking proxy health..."
if ! curl -fsS "$API_URL/health" >/dev/null 2>&1; then
    error "Proxy is not running at $API_URL"
    error "Start it with: docker compose -f docker/docker-compose.yml up -d"
    exit 1
fi
success "Proxy is up"
echo ""

# ── Helper: send one request ──────────────────────────────────────────────────
send_openai_request() {
    local agent_id="$1"
    local agent_name="$2"
    local model="$3"
    local message="$4"
    local description="$5"

    info "Sending: $description (agent=$agent_name, model=$model)"

    HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$OPENAI_ENDPOINT" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer sk-demo-fake-key-govrix-scout-seed" \
        -H "X-govrix-scout-Agent-Id: $agent_id" \
        -H "Agent-Name: $agent_name" \
        --max-time 15 \
        -d "{
            \"model\": \"$model\",
            \"messages\": [{\"role\": \"user\", \"content\": \"$message\"}],
            \"max_tokens\": 100
        }" 2>/dev/null || echo "000")

    if [[ "$HTTP_STATUS" == "000" ]]; then
        warn "  Request timed out (upstream unreachable — this is expected with a demo key)"
    elif [[ "$HTTP_STATUS" == "401" || "$HTTP_STATUS" == "403" ]]; then
        success "  Received $HTTP_STATUS (auth rejected by upstream — event still logged by proxy)"
    elif [[ "$HTTP_STATUS" == "502" || "$HTTP_STATUS" == "503" ]]; then
        success "  Received $HTTP_STATUS (upstream error — event still logged by proxy)"
    else
        success "  HTTP $HTTP_STATUS"
    fi
}

send_anthropic_request() {
    local agent_id="$1"
    local agent_name="$2"
    local model="$3"
    local message="$4"
    local description="$5"

    info "Sending: $description (agent=$agent_name, model=$model)"

    HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$ANTHROPIC_ENDPOINT" \
        -H "Content-Type: application/json" \
        -H "x-api-key: sk-ant-demo-fake-key-govrix-scout-seed" \
        -H "anthropic-version: 2023-06-01" \
        -H "X-govrix-scout-Agent-Id: $agent_id" \
        -H "Agent-Name: $agent_name" \
        --max-time 15 \
        -d "{
            \"model\": \"$model\",
            \"max_tokens\": 100,
            \"messages\": [{\"role\": \"user\", \"content\": \"$message\"}]
        }" 2>/dev/null || echo "000")

    if [[ "$HTTP_STATUS" == "000" ]]; then
        warn "  Request timed out (upstream unreachable — this is expected with a demo key)"
    elif [[ "$HTTP_STATUS" == "401" || "$HTTP_STATUS" == "403" ]]; then
        success "  Received $HTTP_STATUS (auth rejected by upstream — event still logged by proxy)"
    elif [[ "$HTTP_STATUS" == "502" || "$HTTP_STATUS" == "503" ]]; then
        success "  Received $HTTP_STATUS (upstream error — event still logged by proxy)"
    else
        success "  HTTP $HTTP_STATUS"
    fi
}

# ── Send demo requests ────────────────────────────────────────────────────────
info "Sending 5 demo requests across 3 simulated agents..."
echo ""

# Agent 1: research-agent — uses GPT-4o
send_openai_request \
    "research-agent-001" \
    "Research Agent" \
    "gpt-4o" \
    "Summarize the latest developments in transformer architecture research." \
    "Research query (gpt-4o)"

sleep 0.5

# Agent 1 again — builds up session history
send_openai_request \
    "research-agent-001" \
    "Research Agent" \
    "gpt-4o" \
    "What are the main differences between GPT-4 and GPT-4o?" \
    "Follow-up query (gpt-4o)"

sleep 0.5

# Agent 2: code-assistant — uses gpt-4o-mini (cheaper model)
send_openai_request \
    "code-assistant-042" \
    "Code Assistant" \
    "gpt-4o-mini" \
    "Write a Python function to parse JSON from a string with error handling." \
    "Code generation request (gpt-4o-mini)"

sleep 0.5

# Agent 2 again — second request, cost accumulates
send_openai_request \
    "code-assistant-042" \
    "Code Assistant" \
    "gpt-4o-mini" \
    "Review this code and suggest improvements: def foo(x): return x+1" \
    "Code review request (gpt-4o-mini)"

sleep 0.5

# Agent 3: claude-orchestrator — uses Anthropic
send_anthropic_request \
    "orchestrator-007" \
    "Claude Orchestrator" \
    "claude-3-5-haiku-20241022" \
    "List five steps to design a robust REST API." \
    "Planning request (claude-3-5-haiku)"

echo ""
echo "────────────────────────────────────────────────────────────────"
echo ""
info "Checking event count via API..."

EVENT_RESPONSE=$(curl -fsS "$API_URL/api/v1/events?limit=10" 2>/dev/null || echo '{"total":0}')
TOTAL=$(echo "$EVENT_RESPONSE" | grep -o '"total":[0-9]*' | head -1 | cut -d: -f2 || echo "0")

if [[ -n "$TOTAL" && "$TOTAL" -gt 0 ]]; then
    success "Events in store: $TOTAL"
else
    warn "Could not read event count from API (the store may not be connected yet)"
    warn "Check: curl $API_URL/api/v1/events"
fi

echo ""
info "Checking agent count..."
AGENT_RESPONSE=$(curl -fsS "$API_URL/api/v1/agents?limit=10" 2>/dev/null || echo '{"total":0}')
AGENT_TOTAL=$(echo "$AGENT_RESPONSE" | grep -o '"total":[0-9]*' | head -1 | cut -d: -f2 || echo "0")

if [[ -n "$AGENT_TOTAL" && "$AGENT_TOTAL" -gt 0 ]]; then
    success "Agents discovered: $AGENT_TOTAL"
else
    warn "No agents in registry yet (events may be queued, check proxy logs)"
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
echo -e "${GREEN}Demo seed complete!${NC}"
echo ""
echo "Open the dashboard to see the activity:"
echo "  http://localhost:3000"
echo ""
echo "Or query the API directly:"
echo "  curl $API_URL/api/v1/events | jq ."
echo "  curl $API_URL/api/v1/agents | jq ."
echo "────────────────────────────────────────────────────────────────"
