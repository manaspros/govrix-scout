-- Migration 010: Persistent Sessions Table
--
-- Replaces the in-memory session map with a durable PostgreSQL table.
-- On proxy restart, active sessions from the last 30 minutes are reloaded
-- into the in-memory cache.
--
-- session_id is TEXT (derived from agent_id + session token) rather than UUID
-- so that it can be deterministically computed from available request headers.
-- See spec §A.4 for session ID derivation priority.

CREATE TABLE IF NOT EXISTS sessions (
    -- Identity
    session_id      TEXT            PRIMARY KEY,
    agent_id        VARCHAR(255)    NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    trace_id        UUID,

    -- Status
    status          TEXT            NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'idle', 'completed', 'killed')),

    -- Timing
    started_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    last_event_at   TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),

    -- Aggregate counters (updated after each event)
    event_count     INTEGER         NOT NULL DEFAULT 0,
    total_cost_usd  DECIMAL(12, 6)  NOT NULL DEFAULT 0,

    -- Kill switch fields (set when status → 'killed')
    killed_at       TIMESTAMPTZ,
    killed_by       TEXT,
    kill_reason     TEXT
);

CREATE INDEX IF NOT EXISTS sessions_agent_id_idx
    ON sessions (agent_id);

CREATE INDEX IF NOT EXISTS sessions_trace_id_idx
    ON sessions (trace_id) WHERE trace_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS sessions_status_idx
    ON sessions (status);

CREATE INDEX IF NOT EXISTS sessions_last_event_idx
    ON sessions (last_event_at DESC);

COMMENT ON TABLE sessions IS
    'Persistent agent sessions. Survives proxy restarts. In-memory cache is rebuilt from active rows at startup.';
COMMENT ON COLUMN sessions.session_id IS
    'Deterministic ID: X-Session-Id header → agent_id+API_key hash → IP+UA hash.';
COMMENT ON COLUMN sessions.trace_id IS
    'Internal trace UUID for this session (NULL until first event creates the trace).';
COMMENT ON COLUMN sessions.killed_by IS
    'Who triggered the kill: user, budget, risk_threshold, loop_detector, timeout.';
