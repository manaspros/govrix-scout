//! Platform-specific REST API endpoints.
//!
//! Extends Scout's management API with policy and tenant management routes.

use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct PolicySummary {
    total_rules: usize,
    enabled_rules: usize,
    pii_masking_enabled: bool,
}

#[derive(Serialize)]
struct TenantInfo {
    id: String,
    name: String,
    max_agents: u32,
}

async fn list_policies() -> Json<PolicySummary> {
    Json(PolicySummary {
        total_rules: 0,
        enabled_rules: 0,
        pii_masking_enabled: false,
    })
}

async fn list_tenants() -> Json<Vec<TenantInfo>> {
    Json(vec![TenantInfo {
        id: "default".to_string(),
        name: "Default Tenant".to_string(),
        max_agents: 100,
    }])
}

async fn platform_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "platform": true,
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub fn platform_router() -> Router {
    Router::new()
        .route("/api/v1/platform/health", get(platform_health))
        .route("/api/v1/policies", get(list_policies))
        .route("/api/v1/tenants", get(list_tenants))
}
