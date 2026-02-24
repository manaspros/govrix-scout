//! Config read handler.
//!
//! Route map:
//!   GET /api/v1/config — get_config

use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde_json::json;

use crate::api::state::AppState;

/// Return the current sanitized configuration.
///
/// Secrets (database passwords, API keys) are redacted.
///
/// GET /api/v1/config
pub async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cfg = &state.config;

    // Sanitize the database URL: redact password
    let db_url_sanitized = sanitize_db_url(&cfg.database.url);

    Json(json!({
        "data": {
            "proxy": {
                "port": cfg.proxy.port,
                "bind": cfg.proxy.bind,
                "fail_open": cfg.proxy.fail_open,
                "upstream_timeout_ms": cfg.proxy.upstream_timeout_ms,
                "max_body_tee_bytes": cfg.proxy.max_body_tee_bytes,
                "upstream_openai": cfg.proxy.upstream_openai,
                "upstream_anthropic": cfg.proxy.upstream_anthropic,
            },
            "api": {
                "port": cfg.api.port,
                "bind": cfg.api.bind,
                "cors_enabled": cfg.api.cors_enabled,
                "cors_origins": cfg.api.cors_origins,
            },
            "database": {
                "url": db_url_sanitized,
                "max_connections": cfg.database.max_connections,
                "min_connections": cfg.database.min_connections,
                "connect_timeout_secs": cfg.database.connect_timeout_secs,
            },
            "retention": {
                "events_days": cfg.retention.events_days,
                "cost_days": cfg.retention.cost_days,
            },
            "telemetry": {
                "log_level": cfg.telemetry.log_level,
                "prometheus_enabled": cfg.telemetry.prometheus_enabled,
                "prometheus_port": cfg.telemetry.prometheus_port,
            },
            "dashboard": {
                "port": cfg.dashboard.port,
            },
        }
    }))
}

/// Redact the password from a PostgreSQL connection URL.
///
/// `postgres://user:password@host:port/db` → `postgres://user:[redacted]@host:port/db`
fn sanitize_db_url(url: &str) -> String {
    // Try to redact the password portion
    if let Some(at_pos) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let after_scheme = &url[scheme_end + 3..at_pos];
            if let Some(colon_pos) = after_scheme.find(':') {
                let user = &after_scheme[..colon_pos];
                let rest = &url[at_pos..];
                let scheme = &url[..scheme_end + 3];
                return format!("{}{}:[redacted]{}", scheme, user, rest);
            }
        }
    }
    // No password found or unexpected format — return as-is
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_db_url_redacts_password() {
        let url = "postgres://Govrix Scout:supersecret@localhost:5432/Govrix Scout";
        let sanitized = sanitize_db_url(url);
        assert!(!sanitized.contains("supersecret"));
        assert!(sanitized.contains("[redacted]"));
        assert!(sanitized.contains("Govrix Scout"));
        assert!(sanitized.contains("localhost:5432"));
    }

    #[test]
    fn sanitize_db_url_no_password() {
        let url = "postgres://localhost:5432/Govrix Scout";
        let sanitized = sanitize_db_url(url);
        assert_eq!(sanitized, url);
    }

    #[test]
    fn sanitize_db_url_preserves_structure() {
        let url = "postgres://user:pass@db.example.com:5432/mydb";
        let sanitized = sanitize_db_url(url);
        assert!(sanitized.starts_with("postgres://"));
        assert!(sanitized.contains("db.example.com:5432/mydb"));
        assert!(!sanitized.contains("pass"));
    }
}
