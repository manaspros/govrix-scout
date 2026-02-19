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
use govrix_common::license;
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
    let policy_hook: Arc<dyn PolicyHook> = Arc::new(GovrixPolicyHook::new(
        Arc::clone(&policy_engine),
        pii_enabled,
    ));
    tracing::info!(pii_enabled, "policy engine initialized");

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
        version: env!("CARGO_PKG_VERSION"),
        engine: Arc::clone(&policy_engine),
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
