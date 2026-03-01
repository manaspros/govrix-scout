-- Migration 004: Convert events to a TimescaleDB hypertable and apply policies
--
-- This migration MUST run after 001_create_events.sql and after the
-- TimescaleDB extension is installed (done automatically by the
-- timescale/timescaledb Docker image used in docker-compose.yml).
--
-- Policies applied:
--   - 1-day chunk interval  (matches MEMORY.md spec)
--   - 7-day data retention  (OSS tier; commercial = unlimited)
--   - 1-day compression     (segments on agent_id, orders on timestamp)

-- Enable the TimescaleDB extension (idempotent)
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Convert the events table to a hypertable partitioned on timestamp.
-- migrate_data => true preserves any rows already in the table.
SELECT create_hypertable(
    'events',
    'timestamp',
    chunk_time_interval => INTERVAL '1 day',
    migrate_data        => true,
    if_not_exists       => true
);

-- ── Compression ──────────────────────────────────────────────────────────────
-- Compress chunks older than 1 day.
-- Segment on agent_id (keeps per-agent queries fast after decompression).
-- Order on timestamp DESC (most recent rows first within a chunk).
ALTER TABLE events SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'agent_id',
    timescaledb.compress_orderby   = 'timestamp DESC'
);

SELECT add_compression_policy(
    'events',
    compress_after => INTERVAL '1 day',
    if_not_exists  => true
);

-- ── Retention ─────────────────────────────────────────────────────────────────
-- Drop chunks older than 7 days (OSS tier).
-- Commercial tier overrides this via the policy engine.
SELECT add_retention_policy(
    'events',
    drop_after    => INTERVAL '7 days',
    if_not_exists => true
);

COMMENT ON TABLE events IS
    'Core audit log (TimescaleDB hypertable): every agent request/response '
    'intercepted by the Govrix Scout proxy. Partitioned by 1-day chunks, '
    'compressed after 1 day, retained for 7 days (OSS).';
