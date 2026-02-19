use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LicenseTier {
    Community,
    Starter,
    Growth,
    Enterprise,
}

#[derive(Debug, Clone)]
pub struct LicenseInfo {
    pub tier: LicenseTier,
    pub max_agents: u32,
    pub retention_days: u32,
    pub policy_enabled: bool,
    pub pii_masking_enabled: bool,
    pub compliance_enabled: bool,
    pub a2a_identity_enabled: bool,
    pub org_id: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Raw payload decoded from a base64-encoded license key.
#[derive(Debug, Deserialize)]
struct LicensePayload {
    tier: String,
    max_agents: Option<u32>,
    policy_enabled: Option<bool>,
    pii_masking_enabled: Option<bool>,
    a2a_identity_enabled: Option<bool>,
    expires_at: Option<String>,
    org_id: Option<String>,
}

fn community() -> LicenseInfo {
    LicenseInfo {
        tier: LicenseTier::Community,
        max_agents: 5,
        retention_days: 7,
        policy_enabled: false,
        pii_masking_enabled: false,
        compliance_enabled: false,
        a2a_identity_enabled: false,
        org_id: None,
        expires_at: None,
    }
}

/// Decode and validate a license key. Always returns a valid `LicenseInfo`.
///
/// - `None` or empty string returns Community tier.
/// - Invalid base64, malformed JSON, or expired keys fall back to Community
///   with a warning log.
pub fn validate_license(key: Option<&str>) -> LicenseInfo {
    let key = match key {
        Some(k) if !k.is_empty() => k,
        _ => return community(),
    };

    // Decode base64
    let bytes = match STANDARD.decode(key) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("license key is not valid base64: {e}");
            return community();
        }
    };

    // Parse JSON
    let payload: LicensePayload = match serde_json::from_slice(&bytes) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("license key contains malformed JSON: {e}");
            return community();
        }
    };

    // Check expiry
    let expires_at = if let Some(ref ts) = payload.expires_at {
        match ts.parse::<DateTime<Utc>>() {
            Ok(dt) => {
                if dt < Utc::now() {
                    tracing::warn!("license expired at {dt}, downgrading to Community tier");
                    return community();
                }
                Some(dt)
            }
            Err(e) => {
                tracing::warn!("invalid expires_at timestamp: {e}");
                return community();
            }
        }
    } else {
        None
    };

    // Map tier string to enum and set defaults per tier
    let tier_str = payload.tier.to_lowercase();
    let (tier, defaults) = match tier_str.as_str() {
        "starter" => (
            LicenseTier::Starter,
            TierDefaults {
                max_agents: 25,
                retention_days: 30,
                policy_enabled: true,
                pii_masking_enabled: false,
                compliance_enabled: false,
                a2a_identity_enabled: false,
            },
        ),
        "growth" => (
            LicenseTier::Growth,
            TierDefaults {
                max_agents: 100,
                retention_days: 90,
                policy_enabled: true,
                pii_masking_enabled: true,
                compliance_enabled: true,
                a2a_identity_enabled: false,
            },
        ),
        "enterprise" => (
            LicenseTier::Enterprise,
            TierDefaults {
                max_agents: u32::MAX,
                retention_days: 365,
                policy_enabled: true,
                pii_masking_enabled: true,
                compliance_enabled: true,
                a2a_identity_enabled: true,
            },
        ),
        _ => {
            tracing::warn!(
                "unknown license tier '{}', falling back to Community",
                payload.tier
            );
            return community();
        }
    };

    LicenseInfo {
        tier,
        max_agents: payload.max_agents.unwrap_or(defaults.max_agents),
        retention_days: defaults.retention_days,
        policy_enabled: payload.policy_enabled.unwrap_or(defaults.policy_enabled),
        pii_masking_enabled: payload
            .pii_masking_enabled
            .unwrap_or(defaults.pii_masking_enabled),
        compliance_enabled: defaults.compliance_enabled,
        a2a_identity_enabled: payload
            .a2a_identity_enabled
            .unwrap_or(defaults.a2a_identity_enabled),
        org_id: payload.org_id,
        expires_at,
    }
}

struct TierDefaults {
    max_agents: u32,
    retention_days: u32,
    policy_enabled: bool,
    pii_masking_enabled: bool,
    compliance_enabled: bool,
    a2a_identity_enabled: bool,
}

/// Helper: encode a JSON value as a base64 license key (useful for tests).
#[cfg(test)]
fn encode_key(json: &serde_json::Value) -> String {
    STANDARD.encode(json.to_string().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn no_key_returns_community() {
        let info = validate_license(None);
        assert_eq!(info.tier, LicenseTier::Community);
        assert_eq!(info.max_agents, 5);
        assert!(!info.policy_enabled);
        assert!(!info.pii_masking_enabled);
        assert!(info.org_id.is_none());
    }

    #[test]
    fn empty_key_returns_community() {
        let info = validate_license(Some(""));
        assert_eq!(info.tier, LicenseTier::Community);
    }

    #[test]
    fn invalid_base64_returns_community() {
        let info = validate_license(Some("not-valid-base64!!!"));
        assert_eq!(info.tier, LicenseTier::Community);
    }

    #[test]
    fn malformed_json_returns_community() {
        let key = STANDARD.encode(b"{ this is not json }");
        let info = validate_license(Some(&key));
        assert_eq!(info.tier, LicenseTier::Community);
    }

    #[test]
    fn valid_starter_key() {
        let key = encode_key(&json!({
            "tier": "starter",
            "max_agents": 50,
            "policy_enabled": true,
            "pii_masking_enabled": false,
            "a2a_identity_enabled": false,
            "expires_at": "2027-01-01T00:00:00Z",
            "org_id": "org-123"
        }));
        let info = validate_license(Some(&key));
        assert_eq!(info.tier, LicenseTier::Starter);
        assert_eq!(info.max_agents, 50);
        assert!(info.policy_enabled);
        assert!(!info.pii_masking_enabled);
        assert_eq!(info.org_id.as_deref(), Some("org-123"));
        assert!(info.expires_at.is_some());
    }

    #[test]
    fn expired_key_returns_community() {
        let key = encode_key(&json!({
            "tier": "growth",
            "expires_at": "2020-01-01T00:00:00Z",
            "org_id": "org-expired"
        }));
        let info = validate_license(Some(&key));
        assert_eq!(info.tier, LicenseTier::Community);
        assert_eq!(info.max_agents, 5);
    }

    #[test]
    fn enterprise_key_all_features() {
        let key = encode_key(&json!({
            "tier": "enterprise",
            "expires_at": "2099-12-31T23:59:59Z",
            "org_id": "org-big-corp"
        }));
        let info = validate_license(Some(&key));
        assert_eq!(info.tier, LicenseTier::Enterprise);
        assert_eq!(info.max_agents, u32::MAX);
        assert!(info.policy_enabled);
        assert!(info.pii_masking_enabled);
        assert!(info.compliance_enabled);
        assert!(info.a2a_identity_enabled);
        assert_eq!(info.org_id.as_deref(), Some("org-big-corp"));
    }

    #[test]
    fn growth_key_with_defaults() {
        let key = encode_key(&json!({
            "tier": "growth",
            "expires_at": "2099-06-15T00:00:00Z"
        }));
        let info = validate_license(Some(&key));
        assert_eq!(info.tier, LicenseTier::Growth);
        assert_eq!(info.max_agents, 100);
        assert!(info.policy_enabled);
        assert!(info.pii_masking_enabled);
        assert!(info.compliance_enabled);
        assert!(!info.a2a_identity_enabled);
        assert!(info.org_id.is_none());
    }

    #[test]
    fn unknown_tier_returns_community() {
        let key = encode_key(&json!({
            "tier": "platinum",
            "expires_at": "2099-01-01T00:00:00Z"
        }));
        let info = validate_license(Some(&key));
        assert_eq!(info.tier, LicenseTier::Community);
    }
}
