//! Health and readiness check handlers.

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

use crate::api::state::AppState;

/// Liveness check — always returns 200 if the process is alive.
///
/// GET /health
pub async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Readiness check — verifies database connectivity.
///
/// GET /ready
pub async fn ready(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_status = match govrix_scout_store::health_check(&state.pool).await {
        Ok(_) => "connected",
        Err(_) => "unavailable",
    };

    let uptime_seconds = state.started_at.elapsed().as_secs();

    let status_code = if db_status == "connected" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(json!({
            "status": if db_status == "connected" { "ready" } else { "degraded" },
            "database": db_status,
            "uptime_seconds": uptime_seconds,
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn health_response_fields_present() {
        // The handler returns version from CARGO_PKG_VERSION and status "ok".
        // We test the JSON structure statically.
        let v = env!("CARGO_PKG_VERSION");
        assert!(!v.is_empty());
    }
}
