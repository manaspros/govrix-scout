//! Proxy server — hyper-based transparent proxy hot path.
//!
//! Architecture (from rust-proxy skill):
//! - Uses hyper directly, NOT axum, for the hot path
//! - SSE responses stream through without buffering
//! - Request body tee: read once, clone bytes for analysis
//! - All analysis is fire-and-forget from the forwarding path
//! - Fail-open: internal errors never block client traffic

pub mod agent_detect;
pub mod handler;
pub mod interceptor;
pub mod streaming;
pub mod upstream;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::events::{EventSender, Metrics};
use crate::policy::{NoOpPolicy, PolicyHook};
use interceptor::InterceptorState;
pub use upstream::UpstreamUrls;

/// Start the hyper proxy server with default upstream URLs.
///
/// Binds to `addr` and serves all incoming connections through `handler::proxy_handler`.
/// The `event_sender` is shared across all connections via Arc.
/// The `metrics` Arc is shared with the management API for real counter reads.
pub async fn serve(
    addr: SocketAddr,
    event_sender: EventSender,
    metrics: Arc<Metrics>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_with_policy(addr, event_sender, metrics, Arc::new(NoOpPolicy)).await
}

/// Start the hyper proxy server with a custom policy hook and default upstream URLs.
pub async fn serve_with_policy(
    addr: SocketAddr,
    event_sender: EventSender,
    metrics: Arc<Metrics>,
    policy_hook: Arc<dyn PolicyHook>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    serve_full(
        addr,
        event_sender,
        metrics,
        policy_hook,
        UpstreamUrls::default(),
    )
    .await
}

/// Start the hyper proxy server with a custom policy hook and custom upstream URLs.
///
/// This is the fully-parameterized entry point. Use this for integration testing
/// with mock upstream servers.
pub async fn serve_full(
    addr: SocketAddr,
    event_sender: EventSender,
    metrics: Arc<Metrics>,
    policy_hook: Arc<dyn PolicyHook>,
    upstream_urls: UpstreamUrls,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use hyper::server::conn::http1;
    use hyper_util::rt::TokioIo;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(addr).await?;
    tracing::info!("proxy listening on {}", addr);

    // Shared interceptor state — one instance for the whole server
    let state = Arc::new(InterceptorState::with_upstream_urls(
        event_sender,
        metrics,
        policy_hook,
        upstream_urls,
    ));

    loop {
        let (stream, peer_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);

        // Clone Arc for this connection
        let state_clone = Arc::clone(&state);

        tokio::spawn(async move {
            let svc = hyper::service::service_fn(move |req| {
                let state = Arc::clone(&state_clone);
                handler::proxy_handler(req, peer_addr, state)
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
                // Log but do not propagate — fail-open design
                tracing::debug!("connection error from {}: {}", peer_addr, e);
            }
        });
    }
}
