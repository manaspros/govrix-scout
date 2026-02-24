//! Govrix Scout Store — PostgreSQL + TimescaleDB persistence layer.
//!
//! Provides connection pooling and typed query functions for:
//! - `events` table (TimescaleDB hypertable)
//! - `agents` table (registry)
//! - `cost_daily` materialized view
//! - Data retention management

pub mod agents;
pub mod costs;
pub mod db;
pub mod events;
pub mod retention;

// ── Top-level re-exports ──────────────────────────────────────────────────────

pub use db::{connect, health_check, StorePool};

// Filter types
pub use agents::AgentFilter;
pub use events::EventFilter;

// Re-export commonly used store functions so callers don't need to know sub-modules
pub use agents::{get_agent, list_agents, retire_agent, update_agent_metadata, upsert_agent};
pub use events::{get_event, get_events_for_agent, get_session_events, list_events};

// Cost query types
pub use costs::{CostBreakdownRow, CostSummary, Granularity, GroupBy};
