//! Govrix Platform — AI agent governance server.
//!
//! Embeds the Scout proxy engine and adds:
//! - Policy evaluation hooks (before request forwarding)
//! - PII masking (inline, not just detection)
//! - Multi-tenant isolation
//!
//! Startup: loads Platform config, initializes Scout proxy.

mod api;

use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

/// Forward a request arriving on the mTLS TLS listener to the plain-HTTP Scout proxy.
///
/// This implements TLS termination: Enterprise agents connect via TLS/mTLS on port 4443,
/// and their requests are forwarded to the Scout proxy (port 4000) which handles policy
/// enforcement, budget checks, and event capture.
async fn mtls_forward(req: axum::extract::Request, proxy_port: u16) -> axum::response::Response {
    use axum::body::Body;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    let client = reqwest::Client::new();
    let method_str = req.method().as_str().to_string();
    let uri = req.uri().clone();
    let headers = req.headers().clone();

    let body_bytes = match axum::body::to_bytes(req.into_body(), 32 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("body read error: {e}")).into_response();
        }
    };

    let path_and_query = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let target_url = format!("http://127.0.0.1:{proxy_port}{path_and_query}");

    let method = reqwest::Method::from_bytes(method_str.as_bytes()).unwrap_or(reqwest::Method::GET);

    let mut builder = client.request(method, &target_url).body(body_bytes);
    for (name, value) in headers.iter() {
        // Skip hop-by-hop headers that should not be forwarded.
        let lower = name.as_str().to_lowercase();
        if lower == "host" || lower == "content-length" || lower == "transfer-encoding" {
            continue;
        }
        builder = builder.header(name.as_str(), value.as_bytes());
    }

    match builder.send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let mut builder = axum::response::Response::builder().status(status);
            for (name, value) in resp.headers().iter() {
                let lower = name.as_str().to_lowercase();
                if lower == "transfer-encoding" {
                    continue;
                }
                builder = builder.header(name.as_str(), value.as_bytes());
            }
            let body_bytes = resp.bytes().await.unwrap_or_default();
            builder
                .body(Body::from(body_bytes))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        Err(e) => {
            tracing::warn!(error = %e, target = %target_url, "mTLS forward failed");
            (
                StatusCode::BAD_GATEWAY,
                format!("upstream proxy error: {e}"),
            )
                .into_response()
        }
    }
}

use govrix_common::config::PlatformConfig;
use govrix_identity::{ca::CertificateAuthority, mtls::MtlsConfig};
use govrix_policy::budget::{BudgetLimit, BudgetTracker};
use govrix_policy::engine::PolicyEngine;
use govrix_policy::hook::GovrixPolicyHook;
use govrix_scout_common::config::Config;
use govrix_scout_proxy::{api as scout_api, events, policy::PolicyHook, proxy};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    // Priority: RUST_LOG (fine-grained) > GOVRIX_LOG_LEVEL > Govrix_LOG_LEVEL > "info"
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("RUST_LOG")
                .or_else(|_| EnvFilter::try_from_env("GOVRIX_LOG_LEVEL"))
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "Govrix Platform starting"
    );

    // ── Scout configuration ─────────────────────────────────────────────────
    let config_path =
        std::env::var("GOVRIX_CONFIG").unwrap_or_else(|_| "config/govrix.default.toml".to_string());
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

    // ── Identity / mTLS ─────────────────────────────────────────────────────
    let (mtls_config, platform_ca) = if platform_cfg.platform.a2a_identity_enabled {
        let org_name = "govrix";
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
        tracing::info!("mTLS disabled (config)");
        (MtlsConfig::default(), None)
    };
    let mtls_config = Arc::new(mtls_config);

    // ── Tenant registry ──────────────────────────────────────────────────────
    let tenant_registry = Arc::new(govrix_common::tenant_registry::TenantRegistry::new());
    tracing::info!(
        tenants = tenant_registry.count(),
        "tenant registry initialized"
    );

    // ── Database pool ───────────────────────────────────────────────────────
    let pool = match govrix_scout_store::connect(&config.database).await {
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
    let pii_enabled = true;

    // ── Budget tracker ───────────────────────────────────────────────────────
    let mut budget = BudgetTracker::new();

    let effective_token_limit = platform_cfg.platform.global_token_limit;
    let effective_cost_limit = platform_cfg.platform.global_cost_limit_usd;

    if effective_token_limit.is_some() || effective_cost_limit.is_some() {
        budget.set_global_limit(BudgetLimit {
            max_tokens: effective_token_limit,
            max_cost_usd: effective_cost_limit,
        });
    }

    // Per-agent limits from config
    for (agent_id, max_tokens) in &platform_cfg.platform.agent_token_limits {
        budget.set_agent_limit(
            agent_id.clone(),
            govrix_policy::budget::BudgetLimit {
                max_tokens: Some(*max_tokens),
                max_cost_usd: platform_cfg
                    .platform
                    .agent_cost_limits
                    .get(agent_id)
                    .copied(),
            },
        );
    }
    tracing::info!(
        agent_limits = platform_cfg.platform.agent_token_limits.len(),
        "per-agent budget limits loaded"
    );

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
    let mtls_tls_config = if let Some(ref ca) = platform_ca {
        let cfg = axum_server::tls_rustls::RustlsConfig::from_pem(
            ca.ca_cert.as_bytes().to_vec(),
            ca.ca_key.as_bytes().to_vec(),
        )
        .await
        .expect("TLS config from CA PEM");
        tracing::info!("mTLS proxy TLS config built");
        Some(cfg)
    } else {
        None
    };

    if let Some(tls_config) = mtls_tls_config {
        let mtls_addr: SocketAddr = format!("0.0.0.0:{}", platform_cfg.platform.mtls_proxy_port)
            .parse()
            .expect("invalid mTLS port");
        // Forward mTLS traffic to the Scout proxy on its plain-HTTP port.
        // This is a TLS termination proxy: agents with mTLS certs connect here,
        // their requests pass through the same policy enforcement at the Scout proxy.
        let inner_proxy_port = config.proxy.port;
        tracing::info!(
            addr = %mtls_addr,
            inner_port = inner_proxy_port,
            "mTLS TLS proxy starting (forwarding to Scout proxy)"
        );
        tokio::spawn(async move {
            let app = axum::Router::new()
                .route("/health", axum::routing::get(|| async { "mTLS OK" }))
                .fallback(move |req: axum::extract::Request| mtls_forward(req, inner_proxy_port));
            if let Err(e) = axum_server::bind_rustls(mtls_addr, tls_config)
                .serve(app.into_make_service())
                .await
            {
                tracing::error!("mTLS proxy error: {e}");
            }
        });
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
        license_tier: "oss".to_string(),
        max_agents: u32::MAX,
        policy_enabled: true,
        pii_masking_enabled: true,
        compliance_enabled: true,
        a2a_identity_enabled: platform_cfg.platform.a2a_identity_enabled,
        retention_days: 365,
        mtls_enabled: mtls_config.is_mtls_enabled(),
        audit_trail_enabled: pool.is_some(),
        budget_tracking_enabled: effective_token_limit.is_some() || effective_cost_limit.is_some(),
        version: env!("CARGO_PKG_VERSION"),
        engine: Arc::clone(&policy_engine),
        ca: platform_ca.clone(),
        tenant_registry: Arc::clone(&tenant_registry),
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
