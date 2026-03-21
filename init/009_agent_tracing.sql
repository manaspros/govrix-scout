-- Migration 009: Agent Tracing Fields
--
-- Adds tracing columns to the events table:
--   event_kind, span_id, parent_span_id, trace_id, tool_name, tool_args,
--   tool_result, mcp_server, risk_score, external_trace_id
--
-- All columns have DEFAULT values so existing rows remain valid.
-- event_kind uses a CHECK constraint on TEXT (not a PG ENUM) so new kinds
-- can be added without a lock-acquiring ALTER TYPE migration.

-- TimescaleDB requires removing compression before altering hypertables
-- with non-constant defaults. Decompress, alter, then re-enable.
-- Must fully disable compression before adding columns with non-constant defaults
DO $$
BEGIN
    -- Remove compression policy if it exists
    PERFORM remove_compression_policy('events', if_exists => true);
    -- Decompress all chunks
    PERFORM decompress_chunk(c, if_compressed => true)
        FROM show_chunks('events') c;
    -- Disable columnstore / compression setting on the hypertable
    ALTER TABLE events SET (timescaledb.compress = false);
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Compression cleanup skipped: %', SQLERRM;
END $$;

ALTER TABLE events
    ADD COLUMN IF NOT EXISTS event_kind TEXT NOT NULL DEFAULT 'llm.request',
    ADD COLUMN IF NOT EXISTS span_id UUID NOT NULL DEFAULT gen_random_uuid(),
    ADD COLUMN IF NOT EXISTS parent_span_id UUID,
    ADD COLUMN IF NOT EXISTS trace_id UUID,
    ADD COLUMN IF NOT EXISTS tool_name TEXT,
    ADD COLUMN IF NOT EXISTS tool_args JSONB,
    ADD COLUMN IF NOT EXISTS tool_result JSONB,
    ADD COLUMN IF NOT EXISTS mcp_server TEXT,
    ADD COLUMN IF NOT EXISTS risk_score REAL,
    ADD COLUMN IF NOT EXISTS external_trace_id TEXT;

-- Re-enable compression after column additions
ALTER TABLE events SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id',
    timescaledb.compress_orderby = 'timestamp DESC'
);
SELECT add_compression_policy('events', INTERVAL '1 day', if_not_exists => true);

ALTER TABLE events
    ADD CONSTRAINT events_event_kind_check
        CHECK (event_kind IN (
            'llm.request',
            'llm.response',
            'tool.invoke',
            'tool.result',
            'resource.read',
            'resource.write',
            'agent.spawn',
            'agent.complete',
            'memory.read',
            'memory.write',
            'policy.check',
            'policy.block',
            'session.start',
            'session.end',
            'error'
        )) NOT VALID;

CREATE INDEX IF NOT EXISTS events_trace_id_idx
    ON events (trace_id) WHERE trace_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS events_parent_span_id_idx
    ON events (parent_span_id) WHERE parent_span_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS events_tool_name_idx
    ON events (tool_name) WHERE tool_name IS NOT NULL;

CREATE INDEX IF NOT EXISTS events_mcp_server_idx
    ON events (mcp_server) WHERE mcp_server IS NOT NULL;

CREATE INDEX IF NOT EXISTS events_event_kind_idx
    ON events (event_kind);

CREATE INDEX IF NOT EXISTS events_span_id_idx
    ON events (span_id);

COMMENT ON COLUMN events.event_kind IS
    'Structured classification of the event: llm.request, tool.invoke, etc.';
COMMENT ON COLUMN events.span_id IS
    'Per-event unique span identifier (UUID). Links to OTel span hierarchy.';
COMMENT ON COLUMN events.parent_span_id IS
    'Parent span_id — links tool_call spans to the LLM response that triggered them.';
COMMENT ON COLUMN events.trace_id IS
    'Govrix internal trace UUID grouping all spans in one agent task run.';
COMMENT ON COLUMN events.tool_name IS
    'For tool.invoke/tool.result events: the MCP tool name called.';
COMMENT ON COLUMN events.tool_args IS
    'Full JSONB arguments passed to the MCP tool (tools/call params.arguments).';
COMMENT ON COLUMN events.tool_result IS
    'Full JSONB result returned by the MCP tool (tools/call result).';
COMMENT ON COLUMN events.mcp_server IS
    'MCP server that received this tool call (from X-MCP-Server header or URL).';
COMMENT ON COLUMN events.risk_score IS
    'Risk score [0.0, 100.0] computed by the interceptor risk scorer.';
COMMENT ON COLUMN events.external_trace_id IS
    'W3C traceparent trace-id (hex) from inbound requests, for correlation with external tools.';
