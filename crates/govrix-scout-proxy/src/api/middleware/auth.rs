//! Optional bearer token authentication middleware.
//!
//! If `GOVRIX_API_KEY` is set, all requests to the management API must
//! include `Authorization: Bearer <token>`. The following paths bypass auth:
//!   - GET /health
//!   - GET /ready
//!   - GET /metrics
//!
//! If no API key is configured, all requests are allowed (open by default).

use axum::{
    body::Body,
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Auth state injected into the router.
///
/// If `api_key` is `None`, authentication is disabled.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub api_key: Option<String>,
}

impl AuthConfig {
    /// Load from the `GOVRIX_API_KEY` environment variable.
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("GOVRIX_API_KEY").ok(),
        }
    }

    /// Create a config with no authentication (allow all).
    #[allow(dead_code)]
    pub fn open() -> Self {
        Self { api_key: None }
    }
}

/// Paths that bypass bearer token authentication.
const PUBLIC_PATHS: &[&str] = &["/health", "/ready", "/metrics"];

/// Axum middleware function for bearer token auth.
///
/// Usage: `Router::new().layer(axum::middleware::from_fn_with_state(auth_config, auth_middleware))`
pub async fn auth_middleware(
    axum::extract::State(auth): axum::extract::State<AuthConfig>,
    req: Request<Body>,
    next: Next,
) -> Response {
    // Skip auth if no key is configured
    let Some(ref expected_key) = auth.api_key else {
        return next.run(req).await;
    };

    // Skip auth for public paths
    let path = req.uri().path();
    if PUBLIC_PATHS.contains(&path) {
        return next.run(req).await;
    }

    // Extract Authorization header
    let auth_header = req.headers().get(header::AUTHORIZATION);
    match auth_header {
        Some(value) => {
            let header_str = match value.to_str() {
                Ok(s) => s,
                Err(_) => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({ "error": "invalid authorization header encoding" })),
                    )
                        .into_response();
                }
            };

            if let Some(token) = header_str.strip_prefix("Bearer ") {
                if token == expected_key.as_str() {
                    next.run(req).await
                } else {
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({ "error": "invalid API key" })),
                    )
                        .into_response()
                }
            } else {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({ "error": "authorization header must use Bearer scheme" })),
                )
                    .into_response()
            }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "missing Authorization header",
                "hint": "use Authorization: Bearer <GOVRIX_API_KEY>",
            })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_config_open_has_no_key() {
        let config = AuthConfig::open();
        assert!(config.api_key.is_none());
    }

    #[test]
    fn public_paths_are_defined() {
        assert!(PUBLIC_PATHS.contains(&"/health"));
        assert!(PUBLIC_PATHS.contains(&"/ready"));
        assert!(PUBLIC_PATHS.contains(&"/metrics"));
    }

    #[test]
    fn bearer_strip_prefix() {
        let header = "Bearer mytoken123";
        let token = header.strip_prefix("Bearer ");
        assert_eq!(token, Some("mytoken123"));
    }

    #[test]
    fn bearer_strip_prefix_no_match() {
        let header = "Basic dXNlcjpwYXNz";
        let token = header.strip_prefix("Bearer ");
        assert!(token.is_none());
    }
}
