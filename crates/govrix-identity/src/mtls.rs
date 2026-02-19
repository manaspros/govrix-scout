use crate::ca::CertificateAuthority;

/// Configuration for mutual TLS between agents and the platform.
#[derive(Debug, Clone)]
pub struct MtlsConfig {
    pub enabled: bool,
    pub ca: Option<CertificateAuthority>,
    pub require_client_cert: bool,
}

impl MtlsConfig {
    /// Create a new mTLS config. Disabled by default.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            ca: None,
            require_client_cert: enabled,
        }
    }

    /// Create an enabled mTLS config backed by the given CA.
    pub fn with_ca(ca: CertificateAuthority) -> Self {
        Self {
            enabled: true,
            ca: Some(ca),
            require_client_cert: true,
        }
    }

    /// Return `true` if mTLS is enabled.
    pub fn is_mtls_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for MtlsConfig {
    fn default() -> Self {
        Self::new(false)
    }
}

/// Check whether mTLS is enabled in the given config.
pub fn is_mtls_enabled(config: &MtlsConfig) -> bool {
    config.enabled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default() {
        let config = MtlsConfig::default();
        assert!(!is_mtls_enabled(&config));
        assert!(config.ca.is_none());
        assert!(!config.require_client_cert);
    }

    #[test]
    fn enabled_with_ca() {
        let ca = CertificateAuthority::generate("TestOrg").unwrap();
        let config = MtlsConfig::with_ca(ca);
        assert!(is_mtls_enabled(&config));
        assert!(config.ca.is_some());
        assert!(config.require_client_cert);
    }
}
