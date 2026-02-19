use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformConfig {
    #[serde(default)]
    pub platform: PlatformSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformSection {
    #[serde(default)]
    pub policy_enabled: bool,
    #[serde(default)]
    pub pii_masking_enabled: bool,
    #[serde(default)]
    pub a2a_identity_enabled: bool,
    #[serde(default)]
    pub license_key: Option<String>,
    #[serde(default = "default_max_agents")]
    pub max_agents: u32,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    /// Global token budget cap across all agents. `None` means unlimited.
    #[serde(default)]
    pub global_token_limit: Option<u64>,
    /// Global cost budget cap in USD across all agents. `None` means unlimited.
    #[serde(default)]
    pub global_cost_limit_usd: Option<f64>,
    /// Port on which the mTLS proxy listens (Enterprise tier only).
    #[serde(default = "default_mtls_port")]
    pub mtls_proxy_port: u16,
    /// Per-agent token budget. Key = agent_id prefix or "*" for all.
    /// E.g.: { "agent-prod-*" = 100000, "agent-test-*" = 10000 }
    #[serde(default)]
    pub agent_token_limits: std::collections::HashMap<String, u64>,
    /// Per-agent cost budget in USD.
    #[serde(default)]
    pub agent_cost_limits: std::collections::HashMap<String, f64>,
}

fn default_max_agents() -> u32 {
    100
}
fn default_retention_days() -> u32 {
    30
}
fn default_mtls_port() -> u16 {
    4443
}

impl Default for PlatformSection {
    fn default() -> Self {
        Self {
            policy_enabled: false,
            pii_masking_enabled: false,
            a2a_identity_enabled: false,
            license_key: None,
            max_agents: default_max_agents(),
            retention_days: default_retention_days(),
            global_token_limit: None,
            global_cost_limit_usd: None,
            mtls_proxy_port: default_mtls_port(),
            agent_token_limits: std::collections::HashMap::new(),
            agent_cost_limits: std::collections::HashMap::new(),
        }
    }
}

impl PlatformConfig {
    /// Load platform configuration from a TOML file, falling back to defaults
    /// when the file is absent or a field is missing.
    ///
    /// Figment layers: built-in defaults → TOML file (merged, not required).
    pub fn load(path: &str) -> Self {
        Figment::from(Serialized::defaults(PlatformConfig::default()))
            .merge(Toml::file(path))
            .extract()
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_platform_section() {
        let s = PlatformSection::default();
        assert!(!s.policy_enabled);
        assert!(!s.pii_masking_enabled);
        assert_eq!(s.max_agents, 100);
        assert_eq!(s.retention_days, 30);
        assert!(s.license_key.is_none());
        assert!(s.global_token_limit.is_none());
        assert!(s.global_cost_limit_usd.is_none());
    }

    #[test]
    fn load_missing_file_uses_defaults() {
        let cfg = PlatformConfig::load("/nonexistent/govrix.toml");
        assert!(!cfg.platform.policy_enabled);
        assert!(cfg.platform.global_token_limit.is_none());
        assert!(cfg.platform.global_cost_limit_usd.is_none());
    }
}
