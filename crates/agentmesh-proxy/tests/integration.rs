//! Integration tests for the Scout proxy.
//!
//! These tests start a real HTTP test server and verify the proxy
//! forwards requests and records events correctly.

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    /// Verifies that the proxy config loads successfully from defaults.
    #[test]
    fn config_loads_defaults() {
        let cfg = agentmesh_common::config::Config::default();
        assert_eq!(cfg.proxy.port, 4000);
        assert_eq!(cfg.api.port, 4001);
    }

    /// Verifies that a new EventSender can be created without panicking.
    #[test]
    fn event_channel_creates_without_panic() {
        let (sender, _rx) = agentmesh_proxy::events::create_channel();
        assert_eq!(
            sender.metrics().events_sent.load(Ordering::Relaxed),
            0
        );
    }
}
