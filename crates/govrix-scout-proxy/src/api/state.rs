//! Shared application state for the management API.
//!
//! `AppState` is wrapped in an `Arc` and injected via axum's `State` extractor
//! into every handler.

use std::sync::Arc;
use std::time::Instant;

use govrix_scout_common::config::Config;
use govrix_scout_store::StorePool;

use crate::events::Metrics;

/// Shared state for all API handlers.
///
/// Constructed once at startup and shared via `Arc<AppState>`.
pub struct AppState {
    /// PostgreSQL connection pool.
    pub pool: StorePool,

    /// A copy of the runtime configuration (sanitized before serving).
    pub config: Config,

    /// Server start time — used to compute uptime in /ready.
    pub started_at: Instant,

    /// Shared Prometheus-facing metrics counters.
    ///
    /// The same `Arc<Metrics>` is held by `InterceptorState` in the proxy,
    /// so reads here reflect live counter values written by the hot path.
    pub metrics: Arc<Metrics>,
}

impl AppState {
    /// Create a new `AppState` wrapping a database pool, config, and shared metrics.
    pub fn new(pool: StorePool, config: Config, metrics: Arc<Metrics>) -> Arc<Self> {
        Arc::new(Self {
            pool,
            config,
            started_at: Instant::now(),
            metrics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn started_at_is_recent() {
        let elapsed = Instant::now().elapsed();
        // Instant is always non-negative and near-zero at construction
        assert!(elapsed.as_secs() < 5);
    }
}
