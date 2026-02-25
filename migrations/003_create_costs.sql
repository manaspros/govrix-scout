-- Migration 003: Create the cost_daily materialized view
--
-- Aggregates cost, token, and latency data from the events table
-- by day × agent × model × protocol.
--
-- Requires TimescaleDB's time_bucket() function (installed via migration 004).
-- Run migration 004 (create_hypertables) before refreshing this view.

CREATE MATERIALIZED VIEW IF NOT EXISTS cost_daily AS
SELECT
    time_bucket('1 day', timestamp)                                     AS day,
    agent_id,
    COALESCE(model, 'unknown')                                          AS model,
    provider                                                            AS protocol,
    COUNT(*)                                                            AS request_count,
    COALESCE(SUM(input_tokens),  0)                                     AS total_input_tokens,
    COALESCE(SUM(output_tokens), 0)                                     AS total_output_tokens,
    COALESCE(SUM(total_tokens),  0)                                     AS total_tokens,
    COALESCE(SUM(cost_usd),      0)                                     AS total_cost_usd,
    AVG(latency_ms)                                                     AS avg_latency_ms,
    PERCENTILE_CONT(0.50) WITHIN GROUP (ORDER BY latency_ms)            AS p50_latency_ms,
    PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY latency_ms)            AS p95_latency_ms,
    PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY latency_ms)            AS p99_latency_ms
FROM events
GROUP BY day, agent_id, model, provider;

-- Unique index is required for REFRESH MATERIALIZED VIEW CONCURRENTLY
CREATE UNIQUE INDEX IF NOT EXISTS cost_daily_pkey
    ON cost_daily (day, agent_id, model, protocol);

COMMENT ON MATERIALIZED VIEW cost_daily IS
    'Daily cost roll-up by agent, model, and provider protocol. Refresh with REFRESH MATERIALIZED VIEW CONCURRENTLY cost_daily.';
