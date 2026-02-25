//! Management API server (Axum, port 4001).
//!
//! Serves non-hot-path endpoints:
//! - Web dashboard at /dashboard (self-contained HTML, no build step required)
//! - Health checks (/health, /ready)
//! - Prometheus metrics (/metrics)
//! - REST API for events, agents, costs, reports, config
//!
//! Uses axum's Router — acceptable here since this is NOT the proxy hot path.
//!
//! Architecture:
//! - `state`      — shared AppState (pool + config + uptime)
//! - `router`     — axum Router wiring all routes
//! - `handlers/`  — one module per resource (health, events, agents, costs, reports, config)
//! - `middleware/` — cors, auth

pub mod handlers;
pub mod middleware;
pub mod router;
pub mod state;

use std::net::SocketAddr;
use std::sync::Arc;

use govrix_scout_common::config::Config;

use crate::events::Metrics;

/// Start the Axum management API server with database connectivity.
///
/// This is the primary entry point. It connects to PostgreSQL, builds AppState,
/// and serves the full REST API.
///
/// The `metrics` Arc is shared with the proxy server so the `/metrics` endpoint
/// reflects live counter values written by the proxy hot path.
pub async fn serve_with_pool(
    addr: SocketAddr,
    pool: govrix_scout_store::StorePool,
    config: Config,
    metrics: Arc<Metrics>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_with_pool_and_routes(addr, pool, config, metrics, None).await
}

/// Start the management API with extra routes merged into the base router.
///
/// `extra_routes` is an optional axum Router that will be merged into Scout's
/// base router. Use this from Govrix Platform to add `/api/v1/policies`,
/// `/api/v1/tenants`, etc. without modifying Scout internals.
pub async fn serve_with_pool_and_routes(
    addr: SocketAddr,
    pool: govrix_scout_store::StorePool,
    config: Config,
    metrics: Arc<Metrics>,
    extra_routes: Option<axum::Router>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = state::AppState::new(pool, config, metrics);
    let mut app = router::create_router_with_auth(state);

    if let Some(extra) = extra_routes {
        app = app.merge(extra);
    }

    tracing::info!("management API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Start the Axum management API server without a database pool.
///
/// Used during startup before the pool is available, or in tests.
/// Returns stub responses for all database-backed endpoints.
pub async fn serve(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = router::build_router();

    tracing::info!("management API (no-db mode) listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
