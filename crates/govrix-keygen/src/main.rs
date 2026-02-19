use base64::{engine::general_purpose::STANDARD, Engine};
use clap::Parser;
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "govrix-keygen",
    about = "Generate Govrix Platform license keys"
)]
struct Cli {
    #[arg(
        long,
        default_value = "community",
        help = "License tier: community | starter | growth | enterprise"
    )]
    tier: String,

    #[arg(long, default_value = "default")]
    org: String,

    #[arg(long, default_value_t = 0, help = "0 = use tier default")]
    max_agents: u32,

    #[arg(long, help = "Expiry date as RFC3339, e.g. 2027-01-01T00:00:00Z")]
    expires: Option<String>,
}

#[derive(Serialize)]
struct Payload {
    tier: String,
    org_id: String,
    max_agents: u32,
    expires_at: Option<String>,
    policy_enabled: Option<bool>,
    pii_masking_enabled: Option<bool>,
    a2a_identity_enabled: Option<bool>,
}

pub fn generate_key(tier: &str, org: &str, max_agents: u32, expires: Option<&str>) -> String {
    let payload = Payload {
        tier: tier.to_string(),
        org_id: org.to_string(),
        max_agents,
        expires_at: expires.map(|s| s.to_string()),
        policy_enabled: None,
        pii_masking_enabled: None,
        a2a_identity_enabled: None,
    };
    let json = serde_json::to_string(&payload).expect("serialize");
    STANDARD.encode(json.as_bytes())
}

fn main() {
    let cli = Cli::parse();
    let key = generate_key(&cli.tier, &cli.org, cli.max_agents, cli.expires.as_deref());
    println!("{key}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let key = generate_key("growth", "acme-corp", 100, Some("2099-01-01T00:00:00Z"));
        let info = govrix_common::license::validate_license(Some(&key));
        assert_eq!(info.tier, govrix_common::license::LicenseTier::Growth);
        assert_eq!(info.max_agents, 100);
        assert_eq!(info.org_id.as_deref(), Some("acme-corp"));
    }

    #[test]
    fn community_key_falls_back_to_community_tier() {
        // community is not a recognised tier in validate_license — it falls back gracefully
        let key = generate_key("community", "test", 0, None);
        let info = govrix_common::license::validate_license(Some(&key));
        assert_eq!(info.tier, govrix_common::license::LicenseTier::Community);
    }
}
