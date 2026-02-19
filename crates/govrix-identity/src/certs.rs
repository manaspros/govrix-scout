use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rcgen::{CertificateParams, DnType, KeyPair};

use crate::ca::CertificateAuthority;

/// A certificate issued to an individual agent, signed by the CA.
#[derive(Debug, Clone)]
pub struct AgentCertificate {
    pub cert_pem: String,
    pub key_pem: String,
    pub agent_id: String,
    pub expires_at: DateTime<Utc>,
}

/// Issue a new certificate for the given agent, signed by the provided CA.
///
/// The certificate is valid for 90 days with CN set to the `agent_id`.
pub fn issue_agent_cert(ca: &CertificateAuthority, agent_id: &str) -> Result<AgentCertificate> {
    let agent_key = KeyPair::generate().context("failed to generate agent key pair")?;

    let mut params = CertificateParams::default();
    params.distinguished_name.push(DnType::CommonName, agent_id);

    // 90 days from now
    let expires_at = Utc::now() + chrono::Duration::days(90);

    // Set not_after to ~90 days out
    let dt = expires_at;
    params.not_after = rcgen::date_time_ymd(
        dt.format("%Y").to_string().parse::<i32>().unwrap(),
        dt.format("%m").to_string().parse::<u8>().unwrap(),
        dt.format("%d").to_string().parse::<u8>().unwrap(),
    );

    let ca_key = ca.key_pair().context("failed to load CA key pair")?;
    let ca_params = ca.cert_params().context("failed to load CA cert params")?;
    let ca_cert = ca_params
        .self_signed(&ca_key)
        .context("failed to rebuild CA cert for signing")?;

    let agent_cert = params
        .signed_by(&agent_key, &ca_cert, &ca_key)
        .context("failed to sign agent certificate")?;

    Ok(AgentCertificate {
        cert_pem: agent_cert.pem(),
        key_pem: agent_key.serialize_pem(),
        agent_id: agent_id.to_string(),
        expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_cert_for_agent() {
        let ca = CertificateAuthority::generate("TestOrg").unwrap();
        let cert = issue_agent_cert(&ca, "agent-42").unwrap();
        assert_eq!(cert.agent_id, "agent-42");
        assert!(cert.cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(cert.key_pem.contains("BEGIN PRIVATE KEY"));
        assert!(cert.expires_at > Utc::now());
    }

    #[test]
    fn agent_cert_differs_from_ca() {
        let ca = CertificateAuthority::generate("TestOrg").unwrap();
        let cert = issue_agent_cert(&ca, "agent-99").unwrap();
        assert_ne!(cert.cert_pem, ca.ca_cert);
        assert_ne!(cert.key_pem, ca.ca_key);
    }
}
