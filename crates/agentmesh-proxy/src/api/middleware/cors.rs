//! CORS middleware configuration.
//!
//! For development: allows all origins with common headers.
//! In production, origins should be restricted via the config `api.cors_origins`.

use http::Method;
use tower_http::cors::{Any, CorsLayer};

/// Build a permissive CORS layer suitable for development.
///
/// Allows:
/// - All origins (*)
/// - Methods: GET, POST, PUT, DELETE, OPTIONS, PATCH
/// - Headers: content-type, authorization, x-requested-with, accept
/// - Credentials: not included (incompatible with wildcard origins)
pub fn permissive_cors() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::PATCH,
        ])
        .allow_headers(Any)
}

/// Build a restrictive CORS layer from a list of allowed origins.
///
/// Falls back to permissive if the origins list is empty or contains "*".
#[allow(dead_code)]
pub fn restricted_cors(origins: &[String]) -> CorsLayer {
    if origins.is_empty() || origins.iter().any(|o| o == "*") {
        return permissive_cors();
    }

    use http::HeaderValue;
    use tower_http::cors::AllowOrigin;

    let allowed: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();

    if allowed.is_empty() {
        return permissive_cors();
    }

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
            Method::PATCH,
        ])
        .allow_headers(Any)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permissive_cors_builds_without_panic() {
        let _layer = permissive_cors();
    }

    #[test]
    fn restricted_cors_with_wildcard_is_permissive() {
        let origins = vec!["*".to_string()];
        // Should not panic — falls back to permissive
        let _layer = restricted_cors(&origins);
    }

    #[test]
    fn restricted_cors_with_empty_origins_is_permissive() {
        let origins: Vec<String> = vec![];
        let _layer = restricted_cors(&origins);
    }
}
