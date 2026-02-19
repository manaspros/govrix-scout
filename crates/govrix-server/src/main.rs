//! Govrix Platform — commercial AI agent governance server.
//!
//! Embeds the Scout proxy engine and adds:
//! - License validation and tier enforcement
//! - Policy evaluation hooks (before request forwarding)
//! - PII masking (inline, not just detection)
//! - Multi-tenant isolation
//!
//! Startup: loads Platform config, validates license, initializes Scout proxy.

mod api;

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use agentmesh_common::config::Config;
use agentmesh_proxy::{api as scout_api, events, policy::PolicyHook, proxy};
use govrix_common::config::PlatformConfig;
use govrix_common::license::{self, LicenseTier};
use govrix_identity::{ca::CertificateAuthority, mtls::MtlsConfig};
use govrix_policy::budget::{BudgetLimit, BudgetTracker};
use govrix_policy::engine::PolicyEngine;
use govrix_policy::hook::GovrixPolicyHook;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("GOVRIX_LOG_LEVEL")
                .or_else(|_| EnvFilter::try_from_env("AGENTMESH_LOG_LEVEL"))
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Govrix Platform starting"
    );

    // ── License validation ──────────────────────────────────────────────────
    let license_key = std::env::var("GOVRIX_LICENSE_KEY").ok();
    let license_info = license::validate_license(license_key.as_deref());
    tracing::info!(
        tier = ?license_info.tier,
        max_agents = license_info.max_agents,
        policy_enabled = license_info.policy_enabled,
        "license validated"
    );

    // ── Identity / mTLS ─────────────────────────────────────────────────────
    let (mtls_config, platform_ca) = if license_info.a2a_identity_enabled {
        let org_name = license_info.org_id.as_deref().unwrap_or("govrix");
        match CertificateAuthority::generate(org_name) {
            Ok(ca) => {
                tracing::info!(org = org_name, "CA generated, mTLS enabled");
                let mtls = MtlsConfig::with_ca(ca.clone());
                (mtls, Some(Arc::new(ca)))
            }
            Err(e) => {
                tracing::warn!(error = %e, "CA generation failed, falling back to mTLS disabled");
                (MtlsConfig::default(), None)
            }
        }
    } else {
        tracing::info!("mTLS disabled (license tier)");
        (MtlsConfig::default(), None)
    };
    let mtls_config = Arc::new(mtls_config);

    // ── Scout configuration ─────────────────────────────────────────────────
    let config_path = std::env::var("GOVRIX_CONFIG")
        .or_else(|_| std::env::var("AGENTMESH_CONFIG"))
        .unwrap_or_else(|_| "config/agentmesh.default.toml".to_string());
    let config = Config::load_or_default(&config_path);

    tracing::info!(
        proxy_port = config.proxy.port,
        api_port = config.api.port,
        "configuration loaded"
    );

    // ── Platform configuration ───────────────────────────────────────────────
    let govrix_config_path =
        std::env::var("GOVRIX_CONFIG").unwrap_or_else(|_| "config/govrix.default.toml".to_string());
    let platform_cfg = PlatformConfig::load(&govrix_config_path);

    // ── Database pool ───────────────────────────────────────────────────────
    let pool = match agentmesh_store::connect(&config.database).await {
        Ok(p) => {
            tracing::info!("PostgreSQL pool established");
            Some(p)
        }
        Err(e) => {
            tracing::warn!(error = %e, "PostgreSQL unavailable — fail-open mode");
            None
        }
    };

    // ── Event channel ───────────────────────────────────────────────────────
    let (event_sender, event_rx) = events::create_channel();
    let event_metrics = event_sender.metrics().clone();
    let metrics = events::Metrics::new();

    let writer_pool = pool.clone();
    tokio::spawn(events::run_background_writer(
        event_rx,
        event_metrics,
        writer_pool,
        metrics.clone(),
    ));
    tracing::info!(
        capacity = events::EVENT_CHANNEL_CAPACITY,
        "event channel initialized"
    );

    // ── Policy engine ────────────────────────────────────────────────────────
    let policy_engine = Arc::new(RwLock::new(PolicyEngine::new()));
    let pii_enabled = license_info.policy_enabled;

    // ── Budget tracker ───────────────────────────────────────────────────────
    // Priority: explicit config values override tier-based defaults.
    // Community tier gets no global limit (agent count is enforced elsewhere).
    let mut budget = BudgetTracker::new();

    let config_token_limit = platform_cfg.platform.global_token_limit;
    let config_cost_limit = platform_cfg.platform.global_cost_limit_usd;

    // Apply tier-based defaults when no explicit config limit is set.
    let tier_token_limit: Option<u64> = match &license_info.tier {
        LicenseTier::Community => None,
        LicenseTier::Starter => Some(50_000_000), // 50M tokens
        LicenseTier::Growth => Some(500_000_000), // 500M tokens
        LicenseTier::Enterprise => None,          // unlimited by default
    };
    let tier_cost_limit: Option<f64> = match &license_info.tier {
        LicenseTier::Community => None,
        LicenseTier::Starter => Some(500.0),  // $500 USD
        LicenseTier::Growth => Some(5_000.0), // $5,000 USD
        LicenseTier::Enterprise => None,      // unlimited by default
    };

    let effective_token_limit = config_token_limit.or(tier_token_limit);
    let effective_cost_limit = config_cost_limit.or(tier_cost_limit);

    if effective_token_limit.is_some() || effective_cost_limit.is_some() {
        budget.set_global_limit(BudgetLimit {
            max_tokens: effective_token_limit,
            max_cost_usd: effective_cost_limit,
        });
    }

    tracing::info!(
        global_token_limit = ?effective_token_limit,
        global_cost_limit_usd = ?effective_cost_limit,
        "budget tracker configured"
    );

    let policy_hook: Arc<dyn PolicyHook> = Arc::new(
        GovrixPolicyHook::new(Arc::clone(&policy_engine), pii_enabled)
            .with_budget(Arc::new(budget)),
    );
    tracing::info!(pii_enabled, "policy engine initialized");

    // ── mTLS proxy config (TLS listener — v1 wiring) ─────────────────────────
    if let Some(ref ca) = platform_ca {
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem(
            ca.ca_cert.as_bytes().to_vec(),
            ca.ca_key.as_bytes().to_vec(),
        )
        .await
        .expect("TLS config from CA PEM");
        let mtls_addr: std::net::SocketAddr =
            format!("0.0.0.0:{}", platform_cfg.platform.mtls_proxy_port)
                .parse()
                .unwrap();
        tracing::info!(addr = %mtls_addr, "mTLS proxy TLS config built");
        // Full TLS termination integration with Scout's router is a separate spike.
        // For v1 we verify the RustlsConfig compiles and the PEM bytes are valid.
        let _ = tls_config;
    }

    // ── Proxy server ────────────────────────────────────────────────────────
    let proxy_addr: SocketAddr = format!("{}:{}", config.proxy.bind, config.proxy.port)
        .parse()
        .expect("invalid proxy bind address");

    let proxy_event_sender = event_sender.clone();
    let proxy_metrics = metrics.clone();
    let proxy_policy = policy_hook.clone();
    let proxy_handle = tokio::spawn(async move {
        if let Err(e) =
            proxy::serve_with_policy(proxy_addr, proxy_event_sender, proxy_metrics, proxy_policy)
                .await
        {
            tracing::error!("proxy server error: {e}");
        }
    });

    // ── Management API server ───────────────────────────────────────────────
    let api_addr: SocketAddr = format!("{}:{}", config.api.bind, config.api.port)
        .parse()
        .expect("invalid API bind address");

    let api_config = config.clone();
    let api_metrics = metrics.clone();
    let platform_state = Arc::new(api::PlatformState {
        license_tier: license_info.tier.clone(),
        max_agents: license_info.max_agents,
        policy_enabled: license_info.policy_enabled,
        pii_masking_enabled: license_info.pii_masking_enabled,
        mtls_enabled: mtls_config.is_mtls_enabled(),
        version: env!("CARGO_PKG_VERSION"),
        engine: Arc::clone(&policy_engine),
        ca: platform_ca.clone(),
    });
    let platform_routes = api::platform_router(platform_state);
    let api_handle = tokio::spawn(async move {
        let result = match pool {
            Some(p) => {
                scout_api::serve_with_pool_and_routes(
                    api_addr,
                    p,
                    api_config,
                    api_metrics,
                    Some(platform_routes),
                )
                .await
            }
            None => scout_api::serve(api_addr).await,
        };
        if let Err(e) = result {
            tracing::error!("API server error: {e}");
        }
    });

    tracing::info!(
        proxy = %proxy_addr,
        api = %api_addr,
        "Govrix Platform ready"
    );

    // ── Shutdown ────────────────────────────────────────────────────────────
    tokio::select! {
        _ = proxy_handle => {
            tracing::warn!("proxy server exited unexpectedly");
        }
        _ = api_handle => {
            tracing::warn!("API server exited unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received SIGINT, shutting down");
        }
    }

    Ok(())
}
