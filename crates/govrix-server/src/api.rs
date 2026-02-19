//! Platform-specific REST API endpoints.
//!
//! Extends Scout's management API with policy and tenant management routes.

use std::sync::{Arc, RwLock};

use axum::{extract::State, routing::get, Json, Router};
use govrix_common::license::LicenseTier;
use govrix_policy::engine::PolicyEngine;
use serde::Serialize;

/// Shared state available to all platform API handlers.
pub struct PlatformState {
    pub license_tier: LicenseTier,
    pub max_agents: u32,
    pub policy_enabled: bool,
    pub pii_masking_enabled: bool,
    pub version: &'static str,
    pub engine: Arc<RwLock<PolicyEngine>>,
}

#[derive(Serialize)]
struct PolicySummary {
    total_rules: usize,
    enabled_rules: usize,
    policy_enabled: bool,
    pii_masking_enabled: bool,
}

#[derive(Serialize)]
struct TenantInfo {
    id: String,
    name: String,
    max_agents: u32,
}

#[derive(Serialize)]
struct LicenseResponse {
    tier: LicenseTier,
    max_agents: u32,
    features: LicenseFeatures,
}

#[derive(Serialize)]
struct LicenseFeatures {
    policy_enabled: bool,
    pii_masking_enabled: bool,
}

async fn list_policies(State(state): State<Arc<PlatformState>>) -> Json<PolicySummary> {
    let (total_rules, enabled_rules) = state.engine.read().unwrap().rule_count();
    Json(PolicySummary {
        total_rules,
        enabled_rules,
        policy_enabled: state.policy_enabled,
        pii_masking_enabled: state.pii_masking_enabled,
    })
}

async fn list_tenants(State(state): State<Arc<PlatformState>>) -> Json<Vec<TenantInfo>> {
    Json(vec![TenantInfo {
        id: "default".to_string(),
        name: "Default Tenant".to_string(),
        max_agents: state.max_agents,
    }])
}

async fn platform_health(State(state): State<Arc<PlatformState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "platform": true,
        "version": state.version,
        "license_tier": state.license_tier,
    }))
}

async fn license_info(State(state): State<Arc<PlatformState>>) -> Json<LicenseResponse> {
    Json(LicenseResponse {
        tier: state.license_tier.clone(),
        max_agents: state.max_agents,
        features: LicenseFeatures {
            policy_enabled: state.policy_enabled,
            pii_masking_enabled: state.pii_masking_enabled,
        },
    })
}

pub fn platform_router(state: Arc<PlatformState>) -> Router {
    Router::new()
        .route("/api/v1/platform/health", get(platform_health))
        .route("/api/v1/platform/license", get(license_info))
        .route("/api/v1/policies", get(list_policies))
        .route("/api/v1/tenants", get(list_tenants))
        .with_state(state)
}
