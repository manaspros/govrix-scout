//! CostRecord — daily cost aggregation.
//!
//! Corresponds to the `cost_daily` materialized view in MEMORY.md:
//!   time_bucket(1 day), agent_id, model, protocol, request_count,
//!   tokens, cost, latency stats
//!
//! Also used for in-memory cost computation before DB persistence.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A single day's cost aggregation for an agent+model+protocol combination.
///
/// Maps to the `cost_daily` materialized view in TimescaleDB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecord {
    // ── Dimensions ────────────────────────────────────────────────────────────
    /// The calendar day this record covers (UTC).
    pub day: NaiveDate,

    /// The agent that incurred this cost.
    pub agent_id: String,

    /// The model used (e.g. "gpt-4o", "claude-3-5-sonnet-20241022").
    pub model: String,

    /// The provider protocol (e.g. "openai", "anthropic", "mcp").
    pub protocol: String,

    // ── Metrics ───────────────────────────────────────────────────────────────
    /// Total number of requests on this day.
    pub request_count: i64,

    /// Total input tokens consumed.
    pub total_tokens_in: i64,

    /// Total output tokens generated.
    pub total_tokens_out: i64,

    /// Combined total tokens (in + out).
    pub total_tokens: i64,

    /// Total estimated USD cost (DECIMAL 12,8).
    pub total_cost_usd: Decimal,

    // ── Latency stats ─────────────────────────────────────────────────────────
    /// Average latency in milliseconds.
    pub avg_latency_ms: Option<f64>,

    /// p50 latency in milliseconds.
    pub p50_latency_ms: Option<f64>,

    /// p95 latency in milliseconds.
    pub p95_latency_ms: Option<f64>,

    /// p99 latency in milliseconds.
    pub p99_latency_ms: Option<f64>,

    // ── Audit ─────────────────────────────────────────────────────────────────
    /// When this materialized view row was last refreshed.
    pub refreshed_at: DateTime<Utc>,
}

impl CostRecord {
    /// Create a new cost record for the given day/agent/model/protocol.
    pub fn new(
        day: NaiveDate,
        agent_id: impl Into<String>,
        model: impl Into<String>,
        protocol: impl Into<String>,
    ) -> Self {
        Self {
            day,
            agent_id: agent_id.into(),
            model: model.into(),
            protocol: protocol.into(),
            request_count: 0,
            total_tokens_in: 0,
            total_tokens_out: 0,
            total_tokens: 0,
            total_cost_usd: Decimal::ZERO,
            avg_latency_ms: None,
            p50_latency_ms: None,
            p95_latency_ms: None,
            p99_latency_ms: None,
            refreshed_at: Utc::now(),
        }
    }
}

/// Per-model pricing for cost estimation.
/// Prices are in USD per 1M tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model: String,
    pub provider: String,
    /// USD per 1M input tokens.
    pub input_usd_per_1m: Decimal,
    /// USD per 1M output tokens.
    pub output_usd_per_1m: Decimal,
}

impl ModelPricing {
    /// Estimate the cost for the given token counts.
    pub fn estimate_cost(&self, input_tokens: i32, output_tokens: i32) -> Decimal {
        let input_cost =
            self.input_usd_per_1m * Decimal::from(input_tokens) / Decimal::from(1_000_000i64);
        let output_cost =
            self.output_usd_per_1m * Decimal::from(output_tokens) / Decimal::from(1_000_000i64);
        input_cost + output_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn cost_estimation() {
        let pricing = ModelPricing {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            input_usd_per_1m: dec!(2.50),
            output_usd_per_1m: dec!(10.00),
        };
        // 1000 input tokens + 500 output tokens
        let cost = pricing.estimate_cost(1000, 500);
        // input: 2.50 * 1000/1_000_000 = 0.0025
        // output: 10.00 * 500/1_000_000 = 0.005
        // total: 0.0075
        assert_eq!(cost, dec!(0.0075));
    }
}
