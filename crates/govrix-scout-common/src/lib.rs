//! Govrix Scout Common — shared types, models, protocols, and configuration.
//!
//! This crate is the foundation for all other Govrix Scout crates. It defines:
//! - Data models matching the canonical PostgreSQL/ClickHouse schemas
//! - Protocol detection types (OpenAI, Anthropic, MCP, A2A)
//! - Unified error types
//! - TOML configuration parsing with environment variable overrides

pub mod config;
pub mod errors;
pub mod models;
pub mod protocols;

// Re-export commonly used types at the crate root
pub use errors::Govrix ScoutError;
pub use models::{agent::Agent, cost::CostRecord, event::AgentEvent};
pub use protocols::Protocol;
