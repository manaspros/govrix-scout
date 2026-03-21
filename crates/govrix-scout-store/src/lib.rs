//! Govrix Scout Store — PostgreSQL + TimescaleDB persistence layer.
//!
//! Provides connection pooling and typed query functions for:
//! - `events` table (TimescaleDB hypertable)
//! - `agents` table (registry)
//! - `cost_daily` materialized view
//! - `budget_daily` table (budget counter persistence)
//! - Data retention management

pub mod agents;
pub mod budget;
pub mod compliance_queries;
pub mod costs;
pub mod db;
pub mod events;
pub mod projects;
pub mod retention;
pub mod sessions;
pub mod traces;

// ── Top-level re-exports ──────────────────────────────────────────────────────

pub use db::{connect, health_check, StorePool};

// Filter types
pub use agents::AgentFilter;
pub use events::EventFilter;

// Re-export commonly used store functions so callers don't need to know sub-modules
pub use agents::{get_agent, list_agents, retire_agent, update_agent_metadata, upsert_agent};
pub use events::{
    get_agent_runs, get_agent_violations, get_event, get_events_for_agent, get_session_events,
    list_events,
};

// Cost query types
pub use costs::{CostBreakdownRow, CostSummary, Granularity, GroupBy};

// Budget persistence
pub use budget::{
    get_budget_today, get_global_budget_today, list_budget_today, upsert_budget_usage,
};

// Budget config CRUD
pub use budget::{
    delete_budget_config, get_budget_config, get_budget_overview, list_budget_configs,
    upsert_budget_config, BudgetConfigRow,
};
