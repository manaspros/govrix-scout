//! Platform-specific REST API endpoints.
//!
//! Extends Scout's management API with policy and tenant management routes.

use std::sync::{Arc, RwLock};

use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use govrix_common::tenant_registry::TenantRegistry;
use govrix_policy::engine::PolicyEngine;
use serde::{Deserialize, Serialize};

/// Shared state available to all platform API handlers.
pub struct PlatformState {
    pub license_tier: String,
    pub max_agents: u32,
    pub policy_enabled: bool,
    pub pii_masking_enabled: bool,
    pub compliance_enabled: bool,
    pub a2a_identity_enabled: bool,
    pub retention_days: u32,
    pub mtls_enabled: bool,
    pub audit_trail_enabled: bool,
    pub budget_tracking_enabled: bool,
    pub version: &'static str,
    pub engine: Arc<RwLock<PolicyEngine>>,
    pub ca: Option<Arc<govrix_identity::ca::CertificateAuthority>>,
    pub tenant_registry: Arc<TenantRegistry>,
}

#[derive(Serialize)]
struct PolicySummary {
    total_rules: usize,
    enabled_rules: usize,
    policy_enabled: bool,
    pii_masking_enabled: bool,
    message: String,
}

#[derive(Serialize)]
struct TenantInfo {
    id: String,
    name: String,
    max_agents: u32,
}

#[derive(Serialize)]
struct LicenseResponse {
    tier: String,
    max_agents: u32,
    features: LicenseFeatures,
}

#[derive(Serialize)]
struct LicenseFeatures {
    policy_enabled: bool,
    pii_masking_enabled: bool,
    compliance_enabled: bool,
    a2a_identity_enabled: bool,
    retention_days: u32,
}

/// Request body for `POST /api/v1/policies/reload`.
#[derive(Deserialize)]
struct ReloadRequest {
    /// Inline YAML rules (takes priority over `rules_file`).
    rules_yaml: Option<String>,
    /// Path to a YAML file on disk (used when `rules_yaml` is `None`).
    rules_file: Option<String>,
}

/// Response body for a successful policy reload.
#[derive(Serialize)]
struct ReloadResponse {
    rules_loaded: usize,
    message: String,
}

/// Request body for `POST /api/v1/certs/issue`.
#[derive(Deserialize)]
struct IssueCertRequest {
    agent_id: String,
}

/// Response body for a successfully issued agent certificate.
#[derive(Serialize)]
struct IssueCertResponse {
    cert_pem: String,
    key_pem: String,
    expires_at: String,
}

async fn list_policies(State(state): State<Arc<PlatformState>>) -> Json<PolicySummary> {
    let (total_rules, enabled_rules) = state.engine.read().unwrap().rule_count();
    Json(PolicySummary {
        total_rules,
        enabled_rules,
        policy_enabled: state.policy_enabled,
        pii_masking_enabled: state.pii_masking_enabled,
        message: "ok".to_string(),
    })
}

async fn list_tenants(State(state): State<Arc<PlatformState>>) -> Json<Vec<TenantInfo>> {
    Json(
        state
            .tenant_registry
            .list()
            .into_iter()
            .map(|t| TenantInfo {
                id: t.id.to_string(),
                name: t.name,
                max_agents: state.max_agents,
            })
            .collect(),
    )
}

#[derive(Deserialize)]
struct CreateTenantRequest {
    name: String,
}

async fn create_tenant(
    State(state): State<Arc<PlatformState>>,
    Json(req): Json<CreateTenantRequest>,
) -> Json<TenantInfo> {
    let t = state.tenant_registry.create(req.name);
    Json(TenantInfo {
        id: t.id.to_string(),
        name: t.name,
        max_agents: state.max_agents,
    })
}

async fn platform_health(State(state): State<Arc<PlatformState>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "platform": true,
        "version": state.version,
        "license_tier": state.license_tier,
        "mtls_enabled": state.mtls_enabled,
    }))
}

async fn license_info(State(state): State<Arc<PlatformState>>) -> Json<LicenseResponse> {
    Json(LicenseResponse {
        tier: state.license_tier.clone(),
        max_agents: state.max_agents,
        features: LicenseFeatures {
            policy_enabled: state.policy_enabled,
            pii_masking_enabled: state.pii_masking_enabled,
            compliance_enabled: state.compliance_enabled,
            a2a_identity_enabled: state.a2a_identity_enabled,
            retention_days: state.retention_days,
        },
    })
}

/// `POST /api/v1/policies/reload`
///
/// Hot-reload policy rules without restarting the server.  The caller must
/// supply *either* `rules_yaml` (inline YAML string) *or* `rules_file` (a
/// filesystem path the server can read).  `rules_yaml` takes priority when
/// both are provided.
///
/// Returns `200 OK` with the number of rules that were loaded, or an
/// appropriate error status on failure.
async fn reload_policies(
    State(state): State<Arc<PlatformState>>,
    Json(req): Json<ReloadRequest>,
) -> Result<Json<ReloadResponse>, (StatusCode, String)> {
    let rules_loaded = if let Some(yaml) = req.rules_yaml {
        state
            .engine
            .write()
            .unwrap()
            .load_rules_from_yaml(&yaml)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to parse rules_yaml: {e}"),
                )
            })?
    } else if let Some(path) = req.rules_file {
        state
            .engine
            .write()
            .unwrap()
            .load_rules_from_file(std::path::Path::new(&path))
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("failed to load rules_file '{path}': {e}"),
                )
            })?
    } else {
        return Err((
            StatusCode::BAD_REQUEST,
            "must provide rules_yaml or rules_file".to_string(),
        ));
    };

    Ok(Json(ReloadResponse {
        rules_loaded,
        message: format!("policy reload successful: {rules_loaded} rule(s) loaded"),
    }))
}

/// `POST /api/v1/certs/issue`
///
/// Issue a new mTLS certificate for the given agent ID, signed by the platform CA.
/// Returns `403 Forbidden` if mTLS / CA is not enabled in the platform config.
async fn issue_cert(
    State(state): State<Arc<PlatformState>>,
    Json(req): Json<IssueCertRequest>,
) -> Result<Json<IssueCertResponse>, (StatusCode, String)> {
    let ca = state.ca.as_ref().ok_or((
        StatusCode::FORBIDDEN,
        "mTLS not enabled (a2a_identity_enabled is false in config)".to_string(),
    ))?;
    let cert = govrix_identity::certs::issue_agent_cert(ca, &req.agent_id)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(IssueCertResponse {
        cert_pem: cert.cert_pem,
        key_pem: cert.key_pem,
        expires_at: cert.expires_at.to_rfc3339(),
    }))
}

/// `GET /api/v1/compliance/{framework}`
///
/// Generate a compliance report for the requested framework.
/// Supported values: `soc2`, `eu-ai-act`.
async fn compliance_report(
    State(state): State<Arc<PlatformState>>,
    Path(framework): Path<String>,
) -> Result<Json<govrix_policy::compliance::ComplianceReport>, (StatusCode, String)> {
    use govrix_policy::compliance::ComplianceReport;

    match framework.as_str() {
        "soc2" => Ok(Json(ComplianceReport::soc2(
            state.audit_trail_enabled,
            state.policy_enabled,
            state.pii_masking_enabled,
            state.mtls_enabled,
            state.budget_tracking_enabled,
        ))),
        "eu-ai-act" => Ok(Json(ComplianceReport::eu_ai_act(
            state.audit_trail_enabled,
            state.policy_enabled,
            state.pii_masking_enabled,
            state.budget_tracking_enabled,
        ))),
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("unsupported framework '{other}'; supported: soc2, eu-ai-act"),
        )),
    }
}

/// `GET /api/v1/sessions/demo`
///
/// Returns a demo session recording to verify the Session Recorder feature
/// is wired correctly.  Real DB-backed queries will replace this later.
async fn demo_session_recording() -> Json<serde_json::Value> {
    use govrix_policy::session::{SessionEvent, SessionRecording};

    let events = vec![
        SessionEvent {
            timestamp: "2026-02-19T10:00:00Z".to_string(),
            agent_id: "demo-agent".to_string(),
            method: "POST".to_string(),
            upstream_target: "https://api.openai.com/v1/chat/completions".to_string(),
            compliance_tag: "pass:all".to_string(),
            tokens_in: Some(100),
            tokens_out: Some(50),
            cost_usd: Some(0.0015),
            status_code: Some(200),
            lineage_hash: "abc123".to_string(),
        },
        SessionEvent {
            timestamp: "2026-02-19T10:00:05Z".to_string(),
            agent_id: "demo-agent".to_string(),
            method: "POST".to_string(),
            upstream_target: "https://api.openai.com/v1/chat/completions".to_string(),
            compliance_tag: "pass:all".to_string(),
            tokens_in: Some(200),
            tokens_out: Some(100),
            cost_usd: Some(0.003),
            status_code: Some(200),
            lineage_hash: "def456".to_string(),
        },
    ];

    let recording = SessionRecording::from_events(
        "sess-demo-001".to_string(),
        "demo-agent".to_string(),
        events,
    );

    Json(serde_json::to_value(&recording).unwrap())
}

/// Bearer token auth middleware for platform routes.
///
/// If `GOVRIX_API_KEY` is set, all requests must include
/// `Authorization: Bearer <token>`.  If the variable is unset (dev mode),
/// all requests are allowed through.
async fn require_api_key(req: Request<Body>, next: Next) -> Response {
    let expected = std::env::var("GOVRIX_API_KEY").unwrap_or_default();
    if expected.is_empty() {
        // No key configured — open access (dev mode).
        return next.run(req).await;
    }

    let auth_header = req.headers().get(header::AUTHORIZATION);
    match auth_header {
        Some(value) => {
            let header_str = match value.to_str() {
                Ok(s) => s,
                Err(_) => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({ "error": "invalid authorization header encoding" })),
                    )
                        .into_response();
                }
            };
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                if token == expected.as_str() {
                    next.run(req).await
                } else {
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({ "error": "invalid API key" })),
                    )
                        .into_response()
                }
            } else {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({ "error": "authorization header must use Bearer scheme" })),
                )
                    .into_response()
            }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "missing Authorization header",
                "hint": "use Authorization: Bearer <GOVRIX_API_KEY>",
            })),
        )
            .into_response(),
    }
}

async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "ok")
}

pub fn platform_router(state: Arc<PlatformState>) -> Router {
    // Protected routes require a valid API key when GOVRIX_API_KEY is set.
    let protected = Router::new()
        .route("/api/v1/platform/health", get(platform_health))
        .route("/api/v1/platform/license", get(license_info))
        .route("/api/v1/policies", get(list_policies))
        .route("/api/v1/policies/reload", post(reload_policies))
        .route("/api/v1/tenants", get(list_tenants).post(create_tenant))
        .route("/api/v1/certs/issue", post(issue_cert))
        .route("/api/v1/compliance/{framework}", get(compliance_report))
        .route("/api/v1/sessions/demo", get(demo_session_recording))
        .layer(middleware::from_fn(require_api_key))
        .with_state(state);

    // /health is unauthenticated so K8s liveness/readiness probes work without a key.
    Router::new()
        .route("/health", get(health_check))
        .merge(protected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use govrix_common::tenant_registry::TenantRegistry;
    use govrix_policy::engine::PolicyEngine;
    use std::sync::{Arc, RwLock};

    fn make_state() -> Arc<PlatformState> {
        Arc::new(PlatformState {
            license_tier: "oss".to_string(),
            max_agents: 10,
            policy_enabled: true,
            pii_masking_enabled: false,
            compliance_enabled: true,
            a2a_identity_enabled: true,
            retention_days: 365,
            mtls_enabled: false,
            audit_trail_enabled: true,
            budget_tracking_enabled: false,
            version: "test",
            engine: Arc::new(RwLock::new(PolicyEngine::new())),
            ca: None,
            tenant_registry: Arc::new(TenantRegistry::new()),
        })
    }

    #[test]
    fn list_tenants_returns_default() {
        let state = make_state();
        let tenants = state.tenant_registry.list();
        assert_eq!(tenants.len(), 1);
        assert_eq!(tenants[0].name, "default");
    }

    #[test]
    fn create_tenant_via_registry() {
        let state = make_state();
        state.tenant_registry.create("acme".to_string());
        let tenants = state.tenant_registry.list();
        assert_eq!(tenants.len(), 2);
    }

    #[test]
    fn make_state_compiles_with_tenant_registry() {
        let state = make_state();
        assert_eq!(state.tenant_registry.count(), 1);
    }

    #[test]
    fn reload_via_yaml_inline() {
        let state = make_state();
        let yaml = r#"
- name: block-openai
  enabled: true
  conditions:
    - field: provider
      operator: equals
      value: openai
  action: block
"#;
        let count = state
            .engine
            .write()
            .unwrap()
            .load_rules_from_yaml(yaml)
            .expect("valid YAML must parse");
        assert_eq!(count, 1);
        assert_eq!(state.engine.read().unwrap().rule_count(), (1, 1));
    }

    #[test]
    fn reload_via_invalid_yaml_returns_error() {
        let state = make_state();
        let bad_yaml = "{ definitely: [not: valid yaml";
        let result = state.engine.write().unwrap().load_rules_from_yaml(bad_yaml);
        assert!(result.is_err(), "invalid YAML must produce an error");
        // Engine unchanged — still empty.
        assert_eq!(state.engine.read().unwrap().rule_count(), (0, 0));
    }

    #[test]
    fn reload_replaces_previous_rules() {
        let state = make_state();

        let yaml_a = r#"
- name: rule-a
  enabled: true
  conditions: []
  action: allow
- name: rule-b
  enabled: false
  conditions: []
  action: block
"#;
        let count_a = state
            .engine
            .write()
            .unwrap()
            .load_rules_from_yaml(yaml_a)
            .unwrap();
        assert_eq!(count_a, 2);
        assert_eq!(state.engine.read().unwrap().rule_count(), (2, 1));

        let yaml_b = r#"
- name: rule-c
  enabled: true
  conditions: []
  action: alert
"#;
        let count_b = state
            .engine
            .write()
            .unwrap()
            .load_rules_from_yaml(yaml_b)
            .unwrap();
        assert_eq!(count_b, 1);
        // Previous two rules must be gone.
        assert_eq!(state.engine.read().unwrap().rule_count(), (1, 1));
    }
}
