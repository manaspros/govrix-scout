-- Migration 002: Create the agents registry table
--
-- Tracks identity, capabilities, and aggregate statistics for every AI agent
-- observed by the Govrix Scout proxy. The primary key is a VARCHAR agent identifier
-- (not UUID) because agent IDs come from headers, API key mappings, or source IP.
--
-- OSS soft limit: 25 agents. Enforced in application logic, not at DB level.

CREATE TABLE IF NOT EXISTS agents (
    -- Identity
    id                  VARCHAR(255)    NOT NULL PRIMARY KEY,
    name                VARCHAR(255),
    description         TEXT,
    agent_type          VARCHAR(50)     NOT NULL DEFAULT 'unknown',
    status              VARCHAR(20)     NOT NULL DEFAULT 'active',

    -- Lifecycle timestamps
    first_seen_at       TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    last_seen_at        TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    last_error_at       TIMESTAMPTZ,

    -- Network identity
    source_ip           INET,
    fingerprint         VARCHAR(64),

    -- API targets observed (JSONB arrays)
    target_apis         JSONB           NOT NULL DEFAULT '[]',
    mcp_servers         JSONB           NOT NULL DEFAULT '[]',

    -- Aggregate statistics (updated on every proxy request via increment_agent_stats)
    total_requests      BIGINT          NOT NULL DEFAULT 0,
    total_tokens_in     BIGINT          NOT NULL DEFAULT 0,
    total_tokens_out    BIGINT          NOT NULL DEFAULT 0,
    total_cost_usd      DECIMAL(16, 8)  NOT NULL DEFAULT 0.0,
    last_model_used     VARCHAR(100),
    error_count         BIGINT          NOT NULL DEFAULT 0,

    -- Labels and metadata
    labels              JSONB           NOT NULL DEFAULT '{}',
    metadata            JSONB           NOT NULL DEFAULT '{}',

    -- Audit timestamps
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE agents IS
    'Agent registry: identity, lifecycle status, and aggregate statistics for every observed AI agent.';
COMMENT ON COLUMN agents.id IS
    'Agent identifier extracted from X-govrix-scout-Agent-Id header, Agent-Name header, API key suffix, or source IP fallback.';
COMMENT ON COLUMN agents.agent_type IS
    'Framework classification: mcp_client, langchain, crewai, autogen, direct_api, a2a, custom, unknown.';
COMMENT ON COLUMN agents.status IS
    'Lifecycle status: active, idle, error, blocked.';
COMMENT ON COLUMN agents.fingerprint IS
    'Composite fingerprint hash for identifying agents without explicit ID (IP + User-Agent + API key prefix).';
COMMENT ON COLUMN agents.source_ip IS
    'Source IP address stored as PostgreSQL INET type for network-aware queries.';

-- Trigger to keep updated_at current on every row update
CREATE OR REPLACE FUNCTION agents_set_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER agents_updated_at
    BEFORE UPDATE ON agents
    FOR EACH ROW
    EXECUTE FUNCTION agents_set_updated_at();
