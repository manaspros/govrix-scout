//! Data models matching the canonical Govrix Scout database schemas.
//!
//! These types correspond exactly to the PostgreSQL schema defined in the
//! govrix-scout-schemas skill. The OSS version uses PostgreSQL + TimescaleDB.

pub mod agent;
pub mod cost;
pub mod event;
pub mod pricing;

pub use agent::Agent;
pub use cost::CostRecord;
pub use event::AgentEvent;
