-- Migration 001: Create the events table (TimescaleDB hypertable)
--
-- This is the core audit log table. Every intercepted agent action lands here.
-- Compliance invariant: session_id, timestamp, lineage_hash, compliance_tag are REQUIRED.
--
-- After creating the table, migration 004 converts it to a TimescaleDB hypertable
-- and applies retention + compression policies.

CREATE TABLE IF NOT EXISTS events (
    -- Identity
    id                  UUID            NOT NULL,
    session_id          UUID            NOT NULL,
    agent_id            VARCHAR(255)    NOT NULL,

    -- Timing (partition key for TimescaleDB)
    timestamp           TIMESTAMPTZ     NOT NULL,
    latency_ms          INTEGER,

    -- Request metadata
    direction           VARCHAR(20)     NOT NULL DEFAULT 'outbound',
    method              VARCHAR(20)     NOT NULL DEFAULT '',
    upstream_target     VARCHAR(1024)   NOT NULL,
    provider            VARCHAR(20)     NOT NULL DEFAULT 'unknown',
    model               VARCHAR(100),

    -- Response metadata
    status_code         INTEGER,
    finish_reason       VARCHAR(50),

    -- Payload storage
    payload             JSONB,
    raw_size_bytes      BIGINT,

    -- Token & cost metrics
    input_tokens        INTEGER,
    output_tokens       INTEGER,
    total_tokens        INTEGER,
    cost_usd            DECIMAL(12, 8),

    -- Governance fields (compliance-first)
    pii_detected        JSONB           NOT NULL DEFAULT '[]',
    tools_called        JSONB           NOT NULL DEFAULT '[]',
    lineage_hash        VARCHAR(64)     NOT NULL,
    compliance_tag      VARCHAR(100)    NOT NULL,
    tags                JSONB           NOT NULL DEFAULT '{}',
    error_message       TEXT,

    -- Audit
    created_at          TIMESTAMPTZ     NOT NULL DEFAULT NOW(),

    -- TimescaleDB requires the partition column to be part of the primary key
    PRIMARY KEY (id, timestamp)
);

COMMENT ON TABLE events IS
    'Core audit log: every agent request/response intercepted by the Govrix Scout proxy.';
COMMENT ON COLUMN events.session_id IS
    'Groups related requests in a single agent conversation session.';
COMMENT ON COLUMN events.lineage_hash IS
    'SHA-256 Merkle-chain hash linking this event to its predecessor.';
COMMENT ON COLUMN events.compliance_tag IS
    'Policy evaluation result, e.g. "pass:all", "warn:cost_budget", "audit:pii".';
COMMENT ON COLUMN events.pii_detected IS
    'Array of PII findings: [{pii_type, location, confidence}]. Never stores actual PII values.';
