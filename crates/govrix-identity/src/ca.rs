use anyhow::{Context, Result};
use rcgen::{CertificateParams, DnType, IsCa, KeyPair, KeyUsagePurpose};
use std::time::Duration;

/// A certificate authority that can sign agent certificates.
#[derive(Debug, Clone)]
pub struct CertificateAuthority {
    pub ca_cert: String,
    pub ca_key: String,
    pub org_name: String,
}

impl CertificateAuthority {
    /// Generate a new self-signed CA certificate.
    ///
    /// The CA cert is valid for 10 years and has key usage set to certificate signing.
    pub fn generate(org_name: &str) -> Result<Self> {
        let key_pair = KeyPair::generate().context("failed to generate CA key pair")?;

        let mut params = CertificateParams::default();
        params
            .distinguished_name
            .push(DnType::CommonName, format!("{org_name} Govrix CA"));
        params
            .distinguished_name
            .push(DnType::OrganizationName, org_name);
        params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];

        // 10 years validity
        let ten_years = Duration::from_secs(10 * 365 * 24 * 60 * 60);
        params.not_after = rcgen::date_time_ymd(2036, 1, 1);
        let _ = ten_years; // validity expressed via not_after

        let cert = params
            .self_signed(&key_pair)
            .context("failed to self-sign CA certificate")?;

        Ok(Self {
            ca_cert: cert.pem(),
            ca_key: key_pair.serialize_pem(),
            org_name: org_name.to_string(),
        })
    }

    /// Load an existing CA from PEM-encoded certificate and key strings.
    pub fn from_pem(cert_pem: &str, key_pem: &str, org_name: &str) -> Result<Self> {
        // Validate that the PEM strings are parseable
        let _key = KeyPair::from_pem(key_pem).context("invalid CA private key PEM")?;

        Ok(Self {
            ca_cert: cert_pem.to_string(),
            ca_key: key_pem.to_string(),
            org_name: org_name.to_string(),
        })
    }

    /// Return a `KeyPair` parsed from the stored PEM key.
    pub(crate) fn key_pair(&self) -> Result<KeyPair> {
        KeyPair::from_pem(&self.ca_key).context("failed to parse CA key pair")
    }

    /// Rebuild `CertificateParams` matching the original CA cert.
    ///
    /// rcgen 0.13 does not support parsing PEM back into params, so we
    /// reconstruct the params from the stored org_name. The resulting
    /// self-signed cert will have the same DN and CA flags.
    pub(crate) fn cert_params(&self) -> Result<CertificateParams> {
        let mut params = CertificateParams::default();
        params
            .distinguished_name
            .push(DnType::CommonName, format!("{} Govrix CA", self.org_name));
        params
            .distinguished_name
            .push(DnType::OrganizationName, &self.org_name);
        params.is_ca = IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        params.not_after = rcgen::date_time_ymd(2036, 1, 1);
        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_ca_succeeds() {
        let ca = CertificateAuthority::generate("TestOrg").unwrap();
        assert!(ca.ca_cert.contains("BEGIN CERTIFICATE"));
        assert!(ca.ca_key.contains("BEGIN PRIVATE KEY"));
        assert_eq!(ca.org_name, "TestOrg");
    }

    #[test]
    fn from_pem_roundtrip() {
        let ca = CertificateAuthority::generate("RoundTrip").unwrap();
        let ca2 = CertificateAuthority::from_pem(&ca.ca_cert, &ca.ca_key, "RoundTrip").unwrap();
        assert_eq!(ca2.ca_cert, ca.ca_cert);
        assert_eq!(ca2.ca_key, ca.ca_key);
    }

    #[test]
    fn pem_output_is_valid() {
        let ca = CertificateAuthority::generate("ValidPEM").unwrap();
        // Verify we can parse the key back
        KeyPair::from_pem(&ca.ca_key).expect("CA key PEM should be valid");
    }
}
