-- Migration 005: Create all query-path indexes
--
-- TimescaleDB automatically creates the composite (id, timestamp) primary key
-- index defined in migration 001.  Every index below supports a specific
-- API query pattern from MEMORY.md.
--
-- Naming convention: idx_<table>_<column(s)>

-- ── events: common filter / sort columns ─────────────────────────────────────

-- Agent timeline — most common query: "show me all events for agent X"
CREATE INDEX IF NOT EXISTS idx_events_agent_id
    ON events (agent_id, timestamp DESC);

-- Session audit trail — "reconstruct session S"
CREATE INDEX IF NOT EXISTS idx_events_session_id
    ON events (session_id, timestamp ASC);

-- Filter by provider (openai / anthropic / mcp / …)
CREATE INDEX IF NOT EXISTS idx_events_provider
    ON events (provider, timestamp DESC);

-- Filter by model name (gpt-4o, claude-3-5-sonnet, …)
CREATE INDEX IF NOT EXISTS idx_events_model
    ON events (model, timestamp DESC)
    WHERE model IS NOT NULL;

-- Cost analysis — sum cost_usd efficiently
CREATE INDEX IF NOT EXISTS idx_events_cost_usd
    ON events (agent_id, timestamp DESC, cost_usd)
    WHERE cost_usd IS NOT NULL;

-- Status-code filtering — find errors quickly
CREATE INDEX IF NOT EXISTS idx_events_status_code
    ON events (status_code, timestamp DESC)
    WHERE status_code IS NOT NULL;

-- JSONB payload full-text search (GIN)
CREATE INDEX IF NOT EXISTS idx_events_payload_gin
    ON events USING GIN (payload jsonb_path_ops)
    WHERE payload IS NOT NULL;

-- JSONB tags filtering (GIN) — e.g. tags @> '{"env":"prod"}'
CREATE INDEX IF NOT EXISTS idx_events_tags_gin
    ON events USING GIN (tags);

-- PII findings search (GIN) — "find all events with PII type EMAIL_ADDRESS"
CREATE INDEX IF NOT EXISTS idx_events_pii_detected_gin
    ON events USING GIN (pii_detected);

-- Lineage chain lookups — follow the Merkle chain
CREATE INDEX IF NOT EXISTS idx_events_lineage_hash
    ON events (lineage_hash);

-- Compliance tag filtering — "show all warn:cost_budget events"
CREATE INDEX IF NOT EXISTS idx_events_compliance_tag
    ON events (compliance_tag, timestamp DESC);

-- ── agents: common filter / sort columns ─────────────────────────────────────

-- Status filter — "list all active agents"
CREATE INDEX IF NOT EXISTS idx_agents_status
    ON agents (status, last_seen_at DESC);

-- Last-seen sort — default dashboard sort order
CREATE INDEX IF NOT EXISTS idx_agents_last_seen_at
    ON agents (last_seen_at DESC);

-- Cost sort — "which agent costs the most?"
CREATE INDEX IF NOT EXISTS idx_agents_total_cost_usd
    ON agents (total_cost_usd DESC);

-- Source IP lookup — used for fingerprint-based agent identification
CREATE INDEX IF NOT EXISTS idx_agents_source_ip
    ON agents (source_ip)
    WHERE source_ip IS NOT NULL;

-- Labels GIN — e.g. labels @> '{"team":"ml"}'
CREATE INDEX IF NOT EXISTS idx_agents_labels_gin
    ON agents USING GIN (labels);

-- ── cost_daily: query support ─────────────────────────────────────────────────
-- The unique index on (day, agent_id, model, protocol) is already created in
-- migration 003 as a precondition for REFRESH CONCURRENTLY.
-- Add an extra index for time-range + agent lookups.

CREATE INDEX IF NOT EXISTS idx_cost_daily_agent_day
    ON cost_daily (agent_id, day DESC);
