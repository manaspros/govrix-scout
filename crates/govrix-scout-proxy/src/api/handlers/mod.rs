//! REST API handler modules.
//!
//! Each handler module corresponds to a resource:
//! - `health`   — liveness / readiness checks
//! - `events`   — event log queries
//! - `agents`   — agent registry CRUD
//! - `costs`    — cost aggregation
//! - `reports`  — report generation (stub)
//! - `config`   — runtime config read

pub mod agents;
pub mod config;
pub mod costs;
pub mod events;
pub mod health;
pub mod reports;
