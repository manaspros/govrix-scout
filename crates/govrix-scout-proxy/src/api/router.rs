//! Axum router — wires all handler modules into a single Router with shared state.
//!
//! Route map (all on the management API port 4001):
//!
//! Dashboard
//!   GET  /dashboard               ← minimal HTML dashboard (no JS build step)
//!
//! Health
//!   GET  /health
//!   GET  /ready
//!   GET  /metrics
//!
//! Events
//!   GET  /api/v1/events
//!   GET  /api/v1/events/sessions/:session_id   ← must come BEFORE /events/:id
//!   GET  /api/v1/events/:id
//!
//! Agents
//!   GET  /api/v1/agents
//!   GET  /api/v1/agents/:id
//!   PUT  /api/v1/agents/:id
//!   POST /api/v1/agents/:id/retire
//!   GET  /api/v1/agents/:id/events
//!
//! Costs
//!   GET  /api/v1/costs/summary
//!   GET  /api/v1/costs/breakdown
//!
//! Reports
//!   GET  /api/v1/reports/types
//!   GET  /api/v1/reports
//!   POST /api/v1/reports/generate
//!
//! Config
//!   GET  /api/v1/config

use std::sync::Arc;

use std::sync::atomic::Ordering;

use axum::{
    extract::State,
    middleware,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower_http::trace::TraceLayer;

use crate::api::{
    handlers,
    middleware::{auth::AuthConfig, cors::permissive_cors},
    state::AppState,
};

/// Build the full management API router with all routes and middleware.
///
/// Routes requiring the database pool use `State<Arc<AppState>>`.
/// Stateless handlers (reports/types, health, metrics) do not require state.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // ── Dashboard ──────────────────────────────────────────────────────
        .route("/dashboard", get(dashboard))
        // ── Health ─────────────────────────────────────────────────────────
        .route("/health", get(handlers::health::health))
        .route("/ready", get(handlers::health::ready))
        .route("/metrics", get(metrics_handler))
        // ── Events ─────────────────────────────────────────────────────────
        // NOTE: /events/sessions/:session_id must be registered before /events/:id
        // so axum matches the literal "sessions" segment first.
        .route("/api/v1/events", get(handlers::events::list_events))
        .route(
            "/api/v1/events/sessions/:session_id",
            get(handlers::events::get_session_events),
        )
        .route("/api/v1/events/:id", get(handlers::events::get_event))
        // ── Agents ─────────────────────────────────────────────────────────
        .route("/api/v1/agents", get(handlers::agents::list_agents))
        .route(
            "/api/v1/agents/:id",
            get(handlers::agents::get_agent).put(handlers::agents::update_agent),
        )
        .route(
            "/api/v1/agents/:id/retire",
            post(handlers::agents::retire_agent),
        )
        .route(
            "/api/v1/agents/:id/events",
            get(handlers::agents::get_agent_events),
        )
        // ── Costs ──────────────────────────────────────────────────────────
        .route("/api/v1/costs/summary", get(handlers::costs::cost_summary))
        .route(
            "/api/v1/costs/breakdown",
            get(handlers::costs::cost_breakdown),
        )
        // ── Reports ────────────────────────────────────────────────────────
        .route("/api/v1/reports/types", get(handlers::reports::list_types))
        .route("/api/v1/reports", get(handlers::reports::list_reports))
        .route(
            "/api/v1/reports/generate",
            post(handlers::reports::generate_report),
        )
        // ── Config ─────────────────────────────────────────────────────────
        .route("/api/v1/config", get(handlers::config::get_config))
        // ── State & middleware ──────────────────────────────────────────────
        .with_state(state)
        .layer(permissive_cors())
        .layer(TraceLayer::new_for_http())
}

/// Build the router with optional bearer-token auth middleware.
///
/// If `GOVRIX_API_KEY` is set, all non-public paths require auth.
pub fn create_router_with_auth(state: Arc<AppState>) -> Router {
    let auth_config = AuthConfig::from_env();

    if auth_config.api_key.is_some() {
        create_router(state.clone()).layer(middleware::from_fn_with_state(
            auth_config,
            crate::api::middleware::auth::auth_middleware,
        ))
    } else {
        create_router(state)
    }
}

// ── Dashboard handler ──────────────────────────────────────────────────────────

/// Serve the minimal HTML dashboard.
///
/// GET /dashboard — returns the self-contained single-page dashboard that polls
/// the Scout REST API every 5 seconds. No build step or Node.js required.
async fn dashboard() -> Html<&'static str> {
    Html(include_str!("../../static/dashboard.html"))
}

// ── Stub handlers ─────────────────────────────────────────────────────────────

/// Prometheus metrics endpoint — reads real atomic counters from shared state.
///
/// GET /metrics
async fn metrics_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let requests = state.metrics.requests_total.load(Ordering::Relaxed);
    let events = state.metrics.events_total.load(Ordering::Relaxed);
    let agents = state.metrics.agents_active.load(Ordering::Relaxed);

    let body = format!(
        "# HELP govrix_scout_requests_total Total proxy requests intercepted\n\
         # TYPE govrix_scout_requests_total counter\n\
         govrix_scout_requests_total {requests}\n\
         # HELP govrix_scout_events_total Total events written to PostgreSQL\n\
         # TYPE govrix_scout_events_total counter\n\
         govrix_scout_events_total {events}\n\
         # HELP govrix_scout_agents_active Distinct agents seen since process start\n\
         # TYPE govrix_scout_agents_active gauge\n\
         govrix_scout_agents_active {agents}\n"
    );

    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

/// Legacy build_router — kept for backwards compatibility with existing mod.rs `serve()`.
///
/// Creates a router with placeholder AppState that uses a placeholder pool.
/// The full implementation is `create_router(state)`.
pub fn build_router() -> Router {
    // Return an empty router with just health — full router requires AppState.
    // This is only used when serve() is called without a pool (legacy path).
    Router::new()
        .route("/dashboard", get(dashboard))
        .route("/health", get(health_no_state))
        .route("/ready", get(ready_no_state))
        .route("/metrics", get(metrics_stub_handler))
        .route("/api/v1/events", get(stub_list))
        .route("/api/v1/events/:id", get(stub_item))
        .route(
            "/api/v1/events/sessions/:session_id",
            get(stub_session_events),
        )
        .route("/api/v1/agents", get(stub_list))
        .route(
            "/api/v1/agents/:id",
            get(stub_item).put(stub_not_implemented),
        )
        .route("/api/v1/agents/:id/retire", post(stub_not_implemented))
        .route("/api/v1/agents/:id/events", get(stub_list))
        .route("/api/v1/costs/summary", get(stub_cost_summary))
        .route("/api/v1/costs/breakdown", get(stub_cost_breakdown))
        .route("/api/v1/reports/types", get(handlers::reports::list_types))
        .route("/api/v1/reports", get(handlers::reports::list_reports))
        .route(
            "/api/v1/reports/generate",
            post(handlers::reports::generate_report),
        )
        .route("/api/v1/config", get(stub_config))
        .layer(permissive_cors())
        .layer(TraceLayer::new_for_http())
}

// ── No-state fallback stubs (legacy path only) ────────────────────────────────

/// Metrics stub for the no-state (no-db) legacy path — returns all zeros.
async fn metrics_stub_handler() -> impl IntoResponse {
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        "# HELP govrix_scout_requests_total Total proxy requests intercepted\n\
         # TYPE govrix_scout_requests_total counter\n\
         govrix_scout_requests_total 0\n\
         # HELP govrix_scout_events_total Total events written to PostgreSQL\n\
         # TYPE govrix_scout_events_total counter\n\
         govrix_scout_events_total 0\n\
         # HELP govrix_scout_agents_active Distinct agents seen since process start\n\
         # TYPE govrix_scout_agents_active gauge\n\
         govrix_scout_agents_active 0\n",
    )
}

async fn health_no_state() -> impl IntoResponse {
    Json(json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn ready_no_state() -> impl IntoResponse {
    Json(json!({ "status": "ready", "database": "unchecked", "uptime_seconds": 0 }))
}

async fn stub_list() -> impl IntoResponse {
    Json(json!({ "data": [], "total": 0 }))
}

async fn stub_item(axum::extract::Path(id): axum::extract::Path<String>) -> impl IntoResponse {
    (
        axum::http::StatusCode::NOT_FOUND,
        Json(json!({ "error": "not found", "id": id })),
    )
}

async fn stub_session_events(
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    Json(json!({ "data": [], "total": 0, "session_id": session_id }))
}

async fn stub_not_implemented() -> impl IntoResponse {
    (
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "database not connected — start with a configured pool" })),
    )
}

async fn stub_cost_summary() -> impl IntoResponse {
    Json(json!({
        "data": {
            "total_cost_usd": 0.0,
            "total_requests": 0,
            "total_input_tokens": 0,
            "total_output_tokens": 0,
        }
    }))
}

async fn stub_cost_breakdown() -> impl IntoResponse {
    Json(json!({ "data": [], "total": 0 }))
}

async fn stub_config() -> impl IntoResponse {
    Json(json!({
        "data": {
            "proxy": { "port": 4000, "fail_open": true },
            "api": { "port": 4001 },
            "retention": { "events_days": 7 },
        }
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn dashboard_html_contains_Govrix Scout() {
        let html = include_str!("../../static/dashboard.html");
        assert!(html.contains("Govrix Scout"));
        assert!(html.contains("/api/v1/agents"));
    }
}
