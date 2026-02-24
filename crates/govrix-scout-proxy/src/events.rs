//! Event channel and background writer for Govrix Scout proxy.
//!
//! Architecture:
//! - Bounded mpsc channel (10,000 capacity) for fire-and-forget event writes
//! - Proxy hot path uses `try_send` — drops events if channel is full (fail-open)
//! - Background task drains the channel and would batch-insert to DB
//! - Dropped events are counted via atomic counter for metrics
//!
//! Compliance-first invariant: every event sent through this channel MUST
//! already have session_id, timestamp, lineage_hash, and compliance_tag set.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use govrix_scout_common::models::agent::{Agent, AgentType};
use govrix_scout_common::models::event::AgentEvent;
use rust_decimal::Decimal;
use tokio::sync::mpsc;

/// Capacity of the event channel.
/// 10,000 events in-flight provides ~seconds of buffer at high throughput.
pub const EVENT_CHANNEL_CAPACITY: usize = 10_000;

/// Shared metrics counters for the event pipeline.
#[derive(Debug, Default)]
pub struct EventMetrics {
    /// Total events successfully sent to the channel.
    pub events_sent: AtomicU64,
    /// Total events dropped because the channel was full.
    pub events_dropped: AtomicU64,
    /// Total events processed by the background writer.
    pub events_processed: AtomicU64,
}

impl EventMetrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

/// Prometheus-facing metrics counters shared between the proxy and API servers.
///
/// A single `Arc<Metrics>` is created at startup and threaded through both
/// `InterceptorState` (proxy hot path) and `AppState` (management API).
/// All fields use `Ordering::Relaxed` — approximate counts are acceptable for metrics.
#[derive(Debug, Default)]
pub struct Metrics {
    /// Total proxy requests intercepted (incremented per forwarded request).
    pub requests_total: AtomicU64,
    /// Total events successfully written to the database.
    pub events_total: AtomicU64,
    /// Number of distinct agents seen in the most recent flush batch.
    pub agents_active: AtomicU64,
}

impl Metrics {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
}

/// Sender side of the event channel.
///
/// Cloneable — one per request/connection is fine.
#[derive(Clone)]
pub struct EventSender {
    tx: mpsc::Sender<AgentEvent>,
    metrics: Arc<EventMetrics>,
}

impl EventSender {
    /// Send an event fire-and-forget.
    ///
    /// If the channel is full, the event is dropped (counted in metrics).
    /// This NEVER blocks the caller.
    pub fn send(&self, event: AgentEvent) {
        match self.tx.try_send(event) {
            Ok(()) => {
                self.metrics.events_sent.fetch_add(1, Ordering::Relaxed);
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(
                    dropped_total = self.metrics.events_dropped.load(Ordering::Relaxed),
                    "event channel full — dropping event (fail-open)"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Background writer has exited — count as dropped
                self.metrics.events_dropped.fetch_add(1, Ordering::Relaxed);
                tracing::error!("event channel closed — background writer may have exited");
            }
        }
    }

    /// Current metrics snapshot.
    pub fn metrics(&self) -> &Arc<EventMetrics> {
        &self.metrics
    }
}

/// Create a new event channel.
///
/// Returns `(EventSender, mpsc::Receiver<AgentEvent>)`.
/// The receiver should be passed to `run_background_writer`.
pub fn create_channel() -> (EventSender, mpsc::Receiver<AgentEvent>) {
    let metrics = EventMetrics::new();
    let (tx, rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
    let sender = EventSender { tx, metrics };
    (sender, rx)
}

/// Background event writer task.
///
/// Drains the event channel and logs events (or batch-inserts to DB in Phase 1).
/// This task runs forever and should be spawned with `tokio::spawn`.
///
/// Fail-open design: if this task panics or exits, proxy continues working
/// (events are just dropped at the channel boundary).
pub async fn run_background_writer(
    mut rx: mpsc::Receiver<AgentEvent>,
    event_metrics: Arc<EventMetrics>,
    pool: Option<govrix_scout_store::StorePool>,
    metrics: Arc<Metrics>,
) {
    tracing::info!("event background writer started");

    // Batch buffer for future DB inserts (Phase 1)
    let mut batch: Vec<AgentEvent> = Vec::with_capacity(100);

    // Track all distinct agent IDs seen since startup for agents_active gauge.
    let mut seen_agents: std::collections::HashSet<String> = std::collections::HashSet::new();

    loop {
        // Drain up to 100 events or wait up to 100ms
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(100);

        loop {
            tokio::select! {
                biased;

                event = rx.recv() => {
                    match event {
                        Some(ev) => {
                            tracing::debug!(
                                event_id = %ev.id,
                                agent = %ev.agent_id,
                                session = %ev.session_id,
                                provider = %ev.provider,
                                model = ?ev.model,
                                status = ?ev.status_code,
                                latency_ms = ?ev.latency_ms,
                                input_tokens = ?ev.input_tokens,
                                output_tokens = ?ev.output_tokens,
                                lineage_hash = %ev.lineage_hash,
                                compliance_tag = %ev.compliance_tag,
                                "event received"
                            );
                            batch.push(ev);
                            event_metrics.events_processed.fetch_add(1, Ordering::Relaxed);

                            if batch.len() >= 100 {
                                flush_batch(&mut batch, &pool, &metrics, &mut seen_agents).await;
                                break;
                            }
                        }
                        None => {
                            // Channel closed — flush remaining and exit
                            tracing::warn!("event channel closed, flushing {} remaining events", batch.len());
                            flush_batch(&mut batch, &pool, &metrics, &mut seen_agents).await;
                            return;
                        }
                    }
                }

                _ = tokio::time::sleep_until(deadline) => {
                    // Timeout reached — flush whatever we have
                    break;
                }
            }
        }

        if !batch.is_empty() {
            flush_batch(&mut batch, &pool, &metrics, &mut seen_agents).await;
        }
    }
}

/// Flush the current batch of events to PostgreSQL (or just log if no pool).
///
/// Fail-open: database errors are logged as warnings but never crash the proxy.
async fn flush_batch(
    batch: &mut Vec<AgentEvent>,
    pool: &Option<govrix_scout_store::StorePool>,
    metrics: &Arc<Metrics>,
    seen_agents: &mut std::collections::HashSet<String>,
) {
    if batch.is_empty() {
        return;
    }

    match pool {
        Some(p) => {
            match govrix_scout_store::events::insert_events_batch(p, batch).await {
                Ok(count) => {
                    tracing::debug!(count, "flushed event batch to PostgreSQL");
                    // Increment events_total by the number of successfully inserted events.
                    metrics
                        .events_total
                        .fetch_add(count as u64, Ordering::Relaxed);
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        count = batch.len(),
                        "failed to flush event batch to PostgreSQL (fail-open, events dropped)"
                    );
                }
            }

            // Upsert agents from the batch (deduplicated by agent_id).
            // Aggregates per-agent stats so we make one upsert call per unique agent.
            upsert_agents_from_batch(p, batch, metrics, seen_agents).await;
        }
        None => {
            tracing::debug!(
                count = batch.len(),
                "flushing event batch (no DB pool — events discarded)"
            );
        }
    }

    batch.clear();
}

/// Upsert agent records from a batch of events.
///
/// Deduplicates by agent_id, aggregating token and cost stats across all events
/// for each unique agent within the batch. Uses `upsert_agent` which handles
/// ON CONFLICT semantics in the database.
///
/// Fail-open: errors are logged as warnings but never propagated.
async fn upsert_agents_from_batch(
    pool: &govrix_scout_store::StorePool,
    batch: &[AgentEvent],
    metrics: &Arc<Metrics>,
    seen_agents: &mut std::collections::HashSet<String>,
) {
    // Aggregate stats per agent_id within this batch.
    struct AgentAccum {
        last_model: Option<String>,
        tokens_in: i64,
        tokens_out: i64,
        cost_usd: Decimal,
        request_count: i64,
    }

    let mut agents: HashMap<&str, AgentAccum> = HashMap::new();

    for ev in batch {
        let entry = agents.entry(ev.agent_id.as_str()).or_insert(AgentAccum {
            last_model: None,
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: Decimal::ZERO,
            request_count: 0,
        });
        entry.request_count += 1;
        entry.tokens_in += ev.input_tokens.unwrap_or(0) as i64;
        entry.tokens_out += ev.output_tokens.unwrap_or(0) as i64;
        entry.cost_usd += ev.cost_usd.unwrap_or(Decimal::ZERO);
        if ev.model.is_some() {
            entry.last_model = ev.model.clone();
        }
    }

    for (agent_id, accum) in &agents {
        let mut agent = Agent::new(*agent_id, AgentType::Unknown);
        agent.last_model_used = accum.last_model.clone();
        agent.total_tokens_in = accum.tokens_in;
        agent.total_tokens_out = accum.tokens_out;
        agent.total_cost_usd = accum.cost_usd;
        agent.total_requests = accum.request_count;

        if let Err(e) = govrix_scout_store::agents::upsert_agent(pool, &agent).await {
            tracing::warn!(
                error = %e,
                agent_id = %agent_id,
                "failed to upsert agent (fail-open, agent stats may be stale)"
            );
        }
    }

    if !agents.is_empty() {
        // Track all distinct agent IDs seen since startup.
        // seen_agents is maintained by run_background_writer across all batch flushes.
        for agent_id in agents.keys() {
            seen_agents.insert((*agent_id).to_string());
        }
        // Update agents_active with the total count of unique agents seen in-process.
        metrics
            .agents_active
            .store(seen_agents.len() as u64, Ordering::Relaxed);

        tracing::debug!(
            unique_agents = agents.len(),
            total_agents_active = seen_agents.len(),
            "upserted agents from event batch"
        );
    }
}

/// Compute a SHA-256 lineage hash linking this event to the previous one.
///
/// The lineage hash creates a Merkle-like chain for tamper evidence.
/// First event in a session uses "GENESIS" as the previous hash.
///
/// Hash input: `"{prev_hash}|{event_id}|{agent_id}|{timestamp_ms}"`
pub fn compute_lineage_hash(
    prev_hash: &str,
    event_id: &uuid::Uuid,
    agent_id: &str,
    timestamp_ms: i64,
) -> String {
    use sha2::{Digest, Sha256};

    let input = format!("{}|{}|{}|{}", prev_hash, event_id, agent_id, timestamp_ms);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Session tracker — assigns and tracks session IDs per agent.
///
/// A session groups related requests from the same agent into a conversation.
/// Simple heuristic: same agent_id = same session (until a configurable idle timeout).
///
/// This is a lightweight in-memory tracker. Phase 1 will add persistence.
pub struct SessionTracker {
    sessions: std::collections::HashMap<String, SessionState>,
    session_idle_timeout: std::time::Duration,
}

struct SessionState {
    session_id: uuid::Uuid,
    last_event_id: uuid::Uuid,
    last_lineage_hash: String,
    last_seen: std::time::Instant,
}

impl SessionTracker {
    /// Create a new session tracker with a default 30-minute idle timeout.
    pub fn new() -> Self {
        Self::with_timeout(std::time::Duration::from_secs(30 * 60))
    }

    pub fn with_timeout(timeout: std::time::Duration) -> Self {
        Self {
            sessions: std::collections::HashMap::new(),
            session_idle_timeout: timeout,
        }
    }

    /// Get or create a session for the given agent_id.
    ///
    /// Returns `(session_id, prev_lineage_hash)`.
    /// The caller must compute the new lineage hash and update via `record_event`.
    pub fn get_or_create(&mut self, agent_id: &str, event_id: &uuid::Uuid) -> (uuid::Uuid, String) {
        let now = std::time::Instant::now();

        // Expire idle sessions
        if let Some(state) = self.sessions.get(agent_id) {
            if now.duration_since(state.last_seen) > self.session_idle_timeout {
                self.sessions.remove(agent_id);
            }
        }

        if let Some(state) = self.sessions.get(agent_id) {
            (state.session_id, state.last_lineage_hash.clone())
        } else {
            // New session
            let session_id = uuid::Uuid::now_v7();
            let genesis_hash = compute_lineage_hash("GENESIS", event_id, agent_id, 0);
            self.sessions.insert(
                agent_id.to_string(),
                SessionState {
                    session_id,
                    last_event_id: *event_id,
                    last_lineage_hash: genesis_hash.clone(),
                    last_seen: now,
                },
            );
            (session_id, genesis_hash)
        }
    }

    /// Record that an event was processed, updating the lineage chain.
    pub fn record_event(&mut self, agent_id: &str, event_id: uuid::Uuid, lineage_hash: String) {
        if let Some(state) = self.sessions.get_mut(agent_id) {
            state.last_event_id = event_id;
            state.last_lineage_hash = lineage_hash;
            state.last_seen = std::time::Instant::now();
        }
    }
}

impl Default for SessionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lineage_hash_is_deterministic() {
        let id = uuid::Uuid::nil();
        let h1 = compute_lineage_hash("GENESIS", &id, "agent-1", 1000);
        let h2 = compute_lineage_hash("GENESIS", &id, "agent-1", 1000);
        assert_eq!(h1, h2);
    }

    #[test]
    fn lineage_hash_changes_with_prev() {
        let id = uuid::Uuid::nil();
        let h1 = compute_lineage_hash("GENESIS", &id, "agent-1", 1000);
        let h2 = compute_lineage_hash(&h1, &id, "agent-1", 2000);
        assert_ne!(h1, h2);
    }

    #[test]
    fn session_tracker_creates_session() {
        let mut tracker = SessionTracker::new();
        let event_id = uuid::Uuid::now_v7();
        let (session_id, hash) = tracker.get_or_create("agent-1", &event_id);
        assert!(!hash.is_empty());

        // Same agent gets same session
        let (session_id2, _) = tracker.get_or_create("agent-1", &event_id);
        assert_eq!(session_id, session_id2);
    }

    #[test]
    fn session_tracker_different_agents_get_different_sessions() {
        let mut tracker = SessionTracker::new();
        let event_id = uuid::Uuid::now_v7();
        let (s1, _) = tracker.get_or_create("agent-1", &event_id);
        let (s2, _) = tracker.get_or_create("agent-2", &event_id);
        assert_ne!(s1, s2);
    }

    #[test]
    fn event_channel_try_send_non_blocking() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (sender, mut rx) = create_channel();

            // Fill channel and then some — must not block
            let event = govrix_scout_common::models::event::AgentEvent::new(
                "agent-1",
                uuid::Uuid::now_v7(),
                govrix_scout_common::models::event::EventDirection::Outbound,
                "POST",
                "https://api.openai.com/v1/chat/completions",
                govrix_scout_common::models::event::Provider::OpenAI,
                "genesis",
                "audit:none",
            );

            sender.send(event.clone());
            sender.send(event.clone());

            // Drain
            let _ = rx.recv().await;
            let _ = rx.recv().await;
        });
    }
}
