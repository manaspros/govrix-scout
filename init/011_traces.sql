-- Migration 011: Traces Table
--
-- A trace represents the full lifecycle of a single top-level agent task:
-- from the first request through all sub-agent delegations to completion.
--
-- Traces are created lazily on the first event for a new session.
-- They are updated asynchronously by the background writer:
--   - On llm.response / tool.result: increment event_count, update total_cost_usd,
--     update peak_risk_score if higher.
--   - On session.end or kill switch: set status='completed' or 'stopped'.
--   - On error with no subsequent events within 60s: set status='failed'.

CREATE TABLE IF NOT EXISTS traces (
    -- Identity
    trace_id            UUID            PRIMARY KEY DEFAULT gen_random_uuid(),
    root_agent_id       VARCHAR(255)    NOT NULL REFERENCES agents(id) ON DELETE CASCADE,

    -- Description (first prompt text, truncated to 500 chars)
    task_description    TEXT,

    -- Status lifecycle
    status              TEXT            NOT NULL DEFAULT 'running'
                            CHECK (status IN ('running', 'completed', 'stopped', 'failed')),
    stopped_by          TEXT,           -- 'circuit_breaker', 'kill_switch', 'user', 'budget'
    error_message       TEXT,

    -- Timing
    started_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    completed_at        TIMESTAMPTZ,
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),

    -- Aggregate metrics (updated by background writer)
    total_cost_usd      DECIMAL(12, 6)  NOT NULL DEFAULT 0,
    peak_risk_score     REAL,
    event_count         INTEGER         NOT NULL DEFAULT 0,
    agent_count         INTEGER         NOT NULL DEFAULT 1,

    -- W3C correlation
    external_trace_id   TEXT,           -- W3C traceparent trace-id (hex) for external tools

    -- Metadata
    metadata            JSONB
);

CREATE INDEX IF NOT EXISTS traces_root_agent_id_idx
    ON traces (root_agent_id);

CREATE INDEX IF NOT EXISTS traces_status_idx
    ON traces (status);

CREATE INDEX IF NOT EXISTS traces_started_at_idx
    ON traces (started_at DESC);

COMMENT ON TABLE traces IS
    'A single top-level agent task lifecycle: from first request to completion or failure.';
COMMENT ON COLUMN traces.root_agent_id IS
    'The originating agent that started this trace (may spawn sub-agents).';
COMMENT ON COLUMN traces.peak_risk_score IS
    'Highest risk_score seen across all events in this trace.';
COMMENT ON COLUMN traces.agent_count IS
    'Number of distinct agents observed within this trace (including sub-agents via A2A).';
COMMENT ON COLUMN traces.external_trace_id IS
    'W3C hex trace-id from inbound traceparent header, for correlation with Datadog/Jaeger.';
