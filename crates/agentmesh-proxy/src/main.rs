//! AgentMesh proxy binary entry point.
//!
//! Starts two servers in the same process:
//! 1. Proxy server (hyper, port 4000) — hot path, intercepts agent traffic
//! 2. Management API server (axum, port 4001) — health, config, REST API
//!
//! Architecture:
//! - Proxy hot path uses hyper directly (NOT axum) for minimal latency overhead
//! - Management API uses axum (routing overhead is acceptable for non-hot-path)
//! - Both servers share a single Tokio runtime and connection pool
//! - Fail-open: if proxy encounters internal errors, traffic still forwards
//!
//! Event pipeline:
//! - Bounded mpsc channel (10,000 capacity) for fire-and-forget event writes
//! - Background writer task drains channel and logs/stores events
//! - Dropped events are counted (channel-full → fail-open)

use std::net::SocketAddr;

use agentmesh_common::config::Config;
use tracing_subscriber::EnvFilter;

mod api;
mod events;
mod policy;
mod proxy;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("AGENTMESH_LOG_LEVEL")
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // ── Configuration ─────────────────────────────────────────────────────────
    let config_path = std::env::var("AGENTMESH_CONFIG")
        .unwrap_or_else(|_| "config/agentmesh.default.toml".to_string());
    let config = Config::load_or_default(&config_path);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        proxy_port = config.proxy.port,
        api_port = config.api.port,
        "AgentMesh starting"
    );

    // ── Database pool ─────────────────────────────────────────────────────────
    // Attempt to connect to PostgreSQL. On failure, fall back to no-db mode
    // so the proxy still starts and forwards traffic (fail-open).
    let pool_result = agentmesh_store::connect(&config.database).await;
    let pool = match pool_result {
        Ok(p) => {
            tracing::info!("PostgreSQL pool established");
            Some(p)
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "PostgreSQL unavailable — API will serve stub responses; proxy continues"
            );
            None
        }
    };

    // ── Event channel ─────────────────────────────────────────────────────────
    // Bounded channel: proxy sends fire-and-forget, background task drains
    let (event_sender, event_rx) = events::create_channel();
    let event_metrics = event_sender.metrics().clone();

    // Shared Prometheus-facing metrics counters
    let metrics = events::Metrics::new();

    // Spawn background event writer (clone pool so API server can still use the original)
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

    // ── Server addresses ──────────────────────────────────────────────────────
    let proxy_addr: SocketAddr = format!("{}:{}", config.proxy.bind, config.proxy.port)
        .parse()
        .expect("invalid proxy bind address");

    let api_addr: SocketAddr = format!("{}:{}", config.api.bind, config.api.port)
        .parse()
        .expect("invalid API bind address");

    // ── Proxy server ──────────────────────────────────────────────────────────
    // Pass the event sender and metrics into the proxy for fire-and-forget event logging
    let proxy_event_sender = event_sender.clone();
    let proxy_metrics = metrics.clone();
    let proxy_handle = tokio::spawn(async move {
        if let Err(e) = proxy::serve(proxy_addr, proxy_event_sender, proxy_metrics).await {
            tracing::error!("proxy server error: {}", e);
        }
    });

    // ── Management API server ─────────────────────────────────────────────────
    let api_config = config.clone();
    let api_metrics = metrics.clone();
    let api_handle = tokio::spawn(async move {
        let result = match pool {
            Some(p) => api::serve_with_pool(api_addr, p, api_config, api_metrics).await,
            None => api::serve(api_addr).await,
        };
        if let Err(e) = result {
            tracing::error!("API server error: {}", e);
        }
    });

    // Wait for either server to exit (both should run forever in normal operation)
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
