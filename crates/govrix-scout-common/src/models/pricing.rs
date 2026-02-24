//! Model pricing table for cost estimation.
//!
//! Provides built-in pricing for common LLM models from OpenAI and Anthropic.
//! Used by the interceptor to estimate per-event cost from token usage.

use rust_decimal_macros::dec;

use super::cost::ModelPricing;

/// Returns the built-in pricing table for common models.
///
/// Prices are in USD per 1M tokens (input / output).
pub fn default_pricing() -> Vec<ModelPricing> {
    vec![
        // ── OpenAI ──────────────────────────────────────────────────────────
        ModelPricing {
            model: "gpt-4o".to_string(),
            provider: "openai".to_string(),
            input_usd_per_1m: dec!(2.50),
            output_usd_per_1m: dec!(10.00),
        },
        ModelPricing {
            model: "gpt-4o-mini".to_string(),
            provider: "openai".to_string(),
            input_usd_per_1m: dec!(0.15),
            output_usd_per_1m: dec!(0.60),
        },
        ModelPricing {
            model: "gpt-4-turbo".to_string(),
            provider: "openai".to_string(),
            input_usd_per_1m: dec!(10.00),
            output_usd_per_1m: dec!(30.00),
        },
        ModelPricing {
            model: "o1".to_string(),
            provider: "openai".to_string(),
            input_usd_per_1m: dec!(15.00),
            output_usd_per_1m: dec!(60.00),
        },
        ModelPricing {
            model: "o1-mini".to_string(),
            provider: "openai".to_string(),
            input_usd_per_1m: dec!(3.00),
            output_usd_per_1m: dec!(12.00),
        },
        ModelPricing {
            model: "o3-mini".to_string(),
            provider: "openai".to_string(),
            input_usd_per_1m: dec!(1.10),
            output_usd_per_1m: dec!(4.40),
        },
        // ── Anthropic ───────────────────────────────────────────────────────
        ModelPricing {
            model: "claude-3-5-sonnet".to_string(),
            provider: "anthropic".to_string(),
            input_usd_per_1m: dec!(3.00),
            output_usd_per_1m: dec!(15.00),
        },
        ModelPricing {
            model: "claude-3-5-haiku".to_string(),
            provider: "anthropic".to_string(),
            input_usd_per_1m: dec!(0.80),
            output_usd_per_1m: dec!(4.00),
        },
        ModelPricing {
            model: "claude-3-opus".to_string(),
            provider: "anthropic".to_string(),
            input_usd_per_1m: dec!(15.00),
            output_usd_per_1m: dec!(75.00),
        },
        ModelPricing {
            model: "claude-sonnet-4".to_string(),
            provider: "anthropic".to_string(),
            input_usd_per_1m: dec!(3.00),
            output_usd_per_1m: dec!(15.00),
        },
        ModelPricing {
            model: "claude-haiku-4-5".to_string(),
            provider: "anthropic".to_string(),
            input_usd_per_1m: dec!(0.80),
            output_usd_per_1m: dec!(4.00),
        },
        ModelPricing {
            model: "claude-opus-4".to_string(),
            provider: "anthropic".to_string(),
            input_usd_per_1m: dec!(15.00),
            output_usd_per_1m: dec!(75.00),
        },
    ]
}

/// Look up pricing for a model by name.
///
/// 1. Exact match against the pricing table.
/// 2. Prefix match: if `model` starts with a known model name, use that pricing
///    (e.g. "gpt-4o-2024-08-06" matches "gpt-4o").
///
/// Prefix matching uses longest-match-first to avoid "gpt-4o" matching before
/// "gpt-4o-mini".
pub fn lookup_pricing(model: &str) -> Option<ModelPricing> {
    let table = default_pricing();

    // Exact match
    if let Some(p) = table.iter().find(|p| p.model == model) {
        return Some(p.clone());
    }

    // Prefix match — longest prefix wins (e.g. "gpt-4o-mini" before "gpt-4o")
    let mut best: Option<&ModelPricing> = None;
    for p in &table {
        if model.starts_with(&p.model) {
            match best {
                Some(current) if current.model.len() >= p.model.len() => {}
                _ => best = Some(p),
            }
        }
    }

    best.cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_known_model() {
        let pricing = lookup_pricing("gpt-4o").expect("should find gpt-4o");
        assert_eq!(pricing.model, "gpt-4o");
        assert_eq!(pricing.provider, "openai");
        assert_eq!(pricing.input_usd_per_1m, dec!(2.50));
        assert_eq!(pricing.output_usd_per_1m, dec!(10.00));
    }

    #[test]
    fn lookup_unknown() {
        assert!(lookup_pricing("some-unknown-model-xyz").is_none());
    }

    #[test]
    fn lookup_prefix_match() {
        // A dated variant should fall back to the base model pricing.
        let pricing = lookup_pricing("gpt-4o-2024-08-06").expect("should prefix-match gpt-4o");
        assert_eq!(pricing.model, "gpt-4o");
        assert_eq!(pricing.input_usd_per_1m, dec!(2.50));
    }

    #[test]
    fn lookup_prefix_match_longest_wins() {
        // "gpt-4o-mini-2024-07-18" should match "gpt-4o-mini", not "gpt-4o".
        let pricing =
            lookup_pricing("gpt-4o-mini-2024-07-18").expect("should prefix-match gpt-4o-mini");
        assert_eq!(pricing.model, "gpt-4o-mini");
        assert_eq!(pricing.input_usd_per_1m, dec!(0.15));
    }

    #[test]
    fn lookup_anthropic_prefix_match() {
        // "claude-3-5-sonnet-20241022" should match "claude-3-5-sonnet".
        let pricing = lookup_pricing("claude-3-5-sonnet-20241022")
            .expect("should prefix-match claude-3-5-sonnet");
        assert_eq!(pricing.model, "claude-3-5-sonnet");
        assert_eq!(pricing.provider, "anthropic");
        assert_eq!(pricing.input_usd_per_1m, dec!(3.00));
    }
}
