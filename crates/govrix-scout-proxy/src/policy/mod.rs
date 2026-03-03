//! Policy engine — compliance enforcement for intercepted agent traffic.
//!
//! This module provides the full policy evaluation pipeline:
//! - `types` — core enums and structs (`PolicyAction`, `PolicyDecision`, etc.)
//! - `pii`   — regex-based PII detection (email, phone, SSN, credit card, IP)
//! - `budget`— in-memory token/cost budget tracking per agent
//! - `loader`— YAML policy configuration loading
//! - `engine`— the `PolicyEngine` struct that ties everything together
//!
//! # PolicyHook trait
//!
//! The [`PolicyHook`] trait is the extension point for downstream consumers
//! (e.g. Govrix Platform). The proxy interceptor calls `evaluate()` on the
//! hook after building each `AgentEvent`, using the returned compliance tag.
//!
//! Default implementation: [`NoOpPolicy`] returns `"audit:none"` for all events.
//!
//! # Compliance-first invariant
//!
//! Every `PolicyDecision` carries a `compliance_tag` in `"{status}:{policy_name}"`
//! format per the compliance-first skill.
//!
//! # Fail-open design
//!
//! If policy evaluation fails (panic, mutex poison, file error), the engine
//! returns `"audit:error"` and traffic continues. Policy failures MUST NOT
//! block agent traffic in any code path.

pub mod budget;
pub mod engine;
pub mod loader;
pub mod pii;
pub mod types;

use govrix_scout_common::models::event::AgentEvent;

pub use engine::PolicyEngine;
pub use types::{AlertSeverity, PolicyAction, PolicyCondition, PolicyDecision, PolicyRule};

// ── PolicyHook trait ─────────────────────────────────────────────────────────

/// Extension point for policy evaluation in the proxy interceptor.
///
/// The proxy calls `evaluate()` after building each `AgentEvent`.
/// The returned string is stored as the event's `compliance_tag`.
///
/// Implementors can also inspect the event and decide to block the request
/// by returning a `PolicyAction::Block` via `check_request()`.
pub trait PolicyHook: Send + Sync {
    /// Evaluate an event and return its compliance tag.
    ///
    /// Called from a fire-and-forget `tokio::spawn` task — must not block.
    /// Default: returns `"audit:none"`.
    fn compliance_tag(&self, event: &AgentEvent) -> String {
        let _ = event;
        "audit:none".to_string()
    }

    /// Check a request before forwarding upstream.
    ///
    /// Returns `None` to allow, or `Some(reason)` to block with a 403.
    /// Default: always allows.
    fn check_request(&self, event: &AgentEvent) -> Option<String> {
        let _ = event;
        None
    }

    /// Record actual token and cost usage after an event completes.
    ///
    /// Called from `log_response_event` after the compliance tag is set.
    /// Implementations should update in-memory budget counters and optionally
    /// persist the delta to the database (fire-and-forget, non-blocking).
    ///
    /// Default: no-op (used by `NoOpPolicy` and other non-budget hooks).
    fn record_usage(
        &self,
        _agent_id: &str,
        _tokens: u64,
        _cost_usd: f64,
        _pool: Option<govrix_scout_store::StorePool>,
    ) {
    }
}

/// Default no-op policy — allows all traffic with `"audit:none"` tag.
pub struct NoOpPolicy;

impl PolicyHook for NoOpPolicy {}
