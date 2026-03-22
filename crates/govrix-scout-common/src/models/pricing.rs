//! Model pricing database for cost estimation.
//!
//! Embeds the LiteLLM pricing database (~1,900 models) at compile time via
//! `include_str!`, parsed once into a `LazyLock<PricingDb>` on first access.
//!
//! Lookup strategy (5 tiers):
//! 1. Exact match in the full model name map
//! 2. Exact match after stripping provider prefix from the query (e.g., "gemini/gemini-2.5-pro" → "gemini-2.5-pro")
//! 3. Exact match in the provider-stripped database map (e.g., user queries "gemini-2.5-pro", DB has "gemini/gemini-2.5-pro")
//! 4. Prefix match — longest DB key that is a prefix of the query (e.g., "gpt-4o-2024-08-06" → "gpt-4o")
//!    Also checks provider-stripped keys.
//!    Also checks reverse prefix — shortest DB key where query is a prefix (e.g., "claude-3-5-sonnet" → "claude-3-5-sonnet-20241022")
//! 5. Provider-based fallback for truly unknown models (conservative mid-range estimates)

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::path::Path;

use super::cost::ModelPricing;

/// The embedded LiteLLM pricing JSON, compiled into the binary.
/// Format: `[{"m":"model_name","i":input_per_token,"o":output_per_token}, ...]`
const LITELLM_JSON: &str = include_str!("litellm_prices.json");

/// Per-token costs for a single model (raw f64, converted to Decimal on output).
#[derive(Debug, Clone, Copy)]
struct PricingEntry {
    input_per_token: f64,
    output_per_token: f64,
}

/// The parsed pricing database, built once on first access.
struct PricingDb {
    /// Full model name → per-token costs.
    exact: HashMap<String, PricingEntry>,
    /// Provider-prefix-stripped model name → per-token costs.
    /// e.g., "gemini-2.5-pro" from "gemini/gemini-2.5-pro".
    /// Only populated for entries that have a "/" in their name.
    stripped: HashMap<String, PricingEntry>,
}

/// Manual override entries for models whose naming conventions differ from
/// LiteLLM's database (e.g., Anthropic dot-notation like "claude-opus-4.5",
/// or common shorthands like "llama-3.1-405b" that LiteLLM only lists under
/// provider-prefixed names).
///
/// Format: (model_name, input_usd_per_1m, output_usd_per_1m)
/// These are stored as USD per 1M tokens and converted to per-token internally.
const MANUAL_OVERRIDES: &[(&str, f64, f64)] = &[
    // Anthropic short names (no date suffix) — LiteLLM only has dated variants
    ("claude-3-5-sonnet", 3.00, 15.00),
    ("claude-3-5-haiku", 0.80, 4.00),
    ("claude-3-opus", 15.00, 75.00),
    ("claude-sonnet-4", 3.00, 15.00),
    ("claude-haiku-4", 1.00, 5.00),
    // Anthropic dot-notation — LiteLLM uses dashes (claude-opus-4-5 vs claude-opus-4.5)
    ("claude-opus-4.5", 5.00, 25.00),
    ("claude-opus-4", 15.00, 75.00),
    // Mistral short names — LiteLLM only has "mistral/mistral-large-latest" etc.
    ("mistral-large", 0.50, 1.50),
    ("mistral-medium", 0.40, 2.00),
    ("mistral-small", 0.10, 0.30),
    ("codestral", 0.30, 0.90),
    // Llama short names — LiteLLM only has provider-prefixed versions
    ("llama-3.3-70b", 0.60, 0.60),
    ("llama-3.1-405b", 3.00, 3.00),
    ("llama-3.1-70b", 0.60, 0.60),
    ("llama-3.1-8b", 0.05, 0.08),
];

/// The global pricing database, initialized on first access.
static DB: LazyLock<PricingDb> = LazyLock::new(|| {
    /// Compact JSON entry from the embedded file.
    #[derive(serde::Deserialize)]
    struct RawEntry {
        m: String,
        i: f64,
        o: f64,
    }

    let entries: Vec<RawEntry> =
        serde_json::from_str(LITELLM_JSON).expect("embedded litellm_prices.json is invalid");

    let capacity = entries.len() + MANUAL_OVERRIDES.len();
    let mut exact = HashMap::with_capacity(capacity);
    let mut stripped: HashMap<String, PricingEntry> = HashMap::new();

    // Load LiteLLM entries first.
    for entry in &entries {
        let pe = PricingEntry {
            input_per_token: entry.i,
            output_per_token: entry.o,
        };
        exact.insert(entry.m.clone(), pe);

        // Build stripped index: remove everything up to and including the last "/".
        if let Some(pos) = entry.m.rfind('/') {
            let bare = &entry.m[pos + 1..];
            if !bare.is_empty() {
                // Only insert if not already present (first entry wins — typically
                // the direct-provider entry, not a gateway/vertex variant).
                stripped.entry(bare.to_string()).or_insert(pe);
            }
        }
    }

    // Insert manual overrides AFTER LiteLLM so they take priority on conflict.
    // (HashMap::insert overwrites existing entries.)
    for &(name, input_per_1m, output_per_1m) in MANUAL_OVERRIDES {
        let pe = PricingEntry {
            input_per_token: input_per_1m / 1_000_000.0,
            output_per_token: output_per_1m / 1_000_000.0,
        };
        exact.insert(name.to_string(), pe);
    }

    PricingDb { exact, stripped }
});

// ── Dynamic Pricing Support (from config/pricing.json) ──────────────────────────

/// Dynamic pricing entries loaded from config/pricing.json at runtime.
/// Stored as model_id → (input_per_1m_usd, output_per_1m_usd).
type DynamicPricingCache = HashMap<String, (f64, f64)>;

/// Global cache for dynamic pricing loaded from config/pricing.json.
/// Protected by Mutex for thread-safe updates at startup.
static DYNAMIC_PRICING: LazyLock<Mutex<DynamicPricingCache>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Load pricing from config/pricing.json and cache it for lookup_pricing().
///
/// Called once at proxy startup. Falls back gracefully if file doesn't exist
/// or parsing fails — the embedded LiteLLM database remains available.
///
/// # Arguments
/// * `pricing_file_path` - Path to config/pricing.json (e.g., "config/pricing.json")
///
/// # Returns
/// * `Ok(count)` - Number of models loaded
/// * `Err(msg)` - Error reason (but pricing.json is optional)
pub fn init_dynamic_pricing(pricing_file_path: &Path) -> Result<usize, String> {
    // Try to read the file.
    let content = match std::fs::read_to_string(pricing_file_path) {
        Ok(c) => c,
        Err(e) => {
            // File doesn't exist or can't be read — not an error, just use static DB.
            return Err(format!(
                "pricing.json not found at {:?} (will use compiled-in LiteLLM DB): {}",
                pricing_file_path, e
            ));
        }
    };

    // Parse JSON schema produced by scripts/pricing/update_pricing.py.
    #[derive(serde::Deserialize)]
    struct PricingJson {
        schema_version: String,
        last_updated: String,
        models_count: usize,
        models: HashMap<String, ModelEntry>,
    }

    #[derive(serde::Deserialize)]
    struct ModelEntry {
        #[serde(default)]
        provider: String,
        input_per_1m_usd: f64,
        output_per_1m_usd: f64,
        #[serde(default)]
        context_window: Option<usize>,
        #[serde(default)]
        notes: Option<String>,
    }

    let pricing_json: PricingJson = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid pricing.json JSON: {}", e))?;

    // Load into the cached HashMap.
    let mut cache = DYNAMIC_PRICING.lock().unwrap();
    cache.clear(); // Replace any previous load in case of reload

    for (model_id, entry) in &pricing_json.models {
        cache.insert(
            model_id.clone(),
            (entry.input_per_1m_usd, entry.output_per_1m_usd),
        );
    }

    let count = cache.len();
    Ok(count)
}

/// Convert a raw `PricingEntry` (per-token f64) into a `ModelPricing` (per-1M Decimal).
fn make_pricing(model: &str, entry: PricingEntry) -> ModelPricing {
    // Convert per-token → per-1M tokens.
    // Use string parsing through Decimal to avoid f64 → Decimal precision issues.
    let input_per_1m = entry.input_per_token * 1_000_000.0;
    let output_per_1m = entry.output_per_token * 1_000_000.0;

    ModelPricing {
        model: model.to_string(),
        provider: infer_provider(model),
        input_usd_per_1m: f64_to_decimal(input_per_1m),
        output_usd_per_1m: f64_to_decimal(output_per_1m),
    }
}

/// Round an f64 to 6 decimal places and convert to Decimal.
/// This avoids precision noise like 2.4999999999 → 2.50.
fn f64_to_decimal(val: f64) -> Decimal {
    // Round to 6 decimal places to eliminate floating-point noise.
    let rounded = (val * 1_000_000.0).round() / 1_000_000.0;
    // Format with enough precision and parse.
    Decimal::from_str_exact(&format!("{rounded:.6}"))
        .unwrap_or_else(|_| Decimal::try_from(rounded).unwrap_or(Decimal::ZERO))
}

/// Infer the provider name from the model string.
fn infer_provider(model: &str) -> String {
    let lower = model.to_lowercase();

    // Check for provider prefix first (e.g., "gemini/gemini-2.5-pro" → "google").
    if let Some(prefix) = lower.split('/').next() {
        match prefix {
            "openai" => return "openai".into(),
            "anthropic" => return "anthropic".into(),
            "gemini" | "vertex_ai" => return "google".into(),
            "mistral" | "azure_ai" => return "mistral".into(),
            "deepseek" => return "deepseek".into(),
            "cohere" | "cohere_chat" => return "cohere".into(),
            "groq" | "fireworks_ai" | "together_ai" | "anyscale" | "replicate" => {
                return prefix.into()
            }
            "bedrock" | "bedrock_converse" => return "aws".into(),
            "azure" => return "azure".into(),
            _ => {}
        }
    }

    // Infer from model name substrings.
    if lower.contains("gpt")
        || lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
    {
        "openai".into()
    } else if lower.contains("claude") {
        "anthropic".into()
    } else if lower.contains("gemini") {
        "google".into()
    } else if lower.contains("mistral")
        || lower.contains("codestral")
        || lower.contains("ministral")
    {
        "mistral".into()
    } else if lower.contains("llama") {
        "meta".into()
    } else if lower.contains("deepseek") {
        "deepseek".into()
    } else if lower.contains("command") {
        "cohere".into()
    } else {
        "unknown".into()
    }
}

/// Look up pricing for a model by name.
///
/// Uses the embedded LiteLLM database (~1,900 models) plus manual overrides.
///
/// 1. **Exact match** against the pricing table.
/// 2. **Exact match** after stripping provider prefix from the query
///    (e.g., "openai/gpt-4o" → try "gpt-4o").
/// 3. **Exact match** in the provider-stripped database index
///    (e.g., user queries "gemini-2.5-pro", DB has "gemini/gemini-2.5-pro").
/// 4. **Prefix match**: longest DB key that is a prefix of the query
///    (e.g., "gpt-4o-2024-08-06" → "gpt-4o"). Also checks stripped keys.
///    **Reverse prefix**: shortest DB key where query is a prefix of the key
///    (e.g., "claude-3-5-sonnet" → "claude-3-5-sonnet-20241022"). Also checks stripped keys.
/// 5. **Provider fallback**: keyword-based conservative mid-range pricing.
///
/// **Lookup order:**
/// 1. Check dynamic pricing from config/pricing.json (if loaded via init_dynamic_pricing)
/// 2. Fall back to embedded LiteLLM database + manual overrides
pub fn lookup_pricing(model: &str) -> Option<ModelPricing> {
    // ── Tier 0: Check dynamic pricing first (from config/pricing.json) ────────
    if let Ok(cache) = DYNAMIC_PRICING.lock() {
        if let Some(&(input_per_1m, output_per_1m)) = cache.get(model) {
            let provider = infer_provider(model);
            return Some(ModelPricing {
                model: model.to_string(),
                provider,
                input_usd_per_1m: f64_to_decimal(input_per_1m),
                output_usd_per_1m: f64_to_decimal(output_per_1m),
            });
        }
    }

    let db = &*DB;

    // ── Tier 1: Exact match ─────────────────────────────────────────────
    if let Some(&entry) = db.exact.get(model) {
        return Some(make_pricing(model, entry));
    }

    // ── Tier 2: Strip provider prefix from query and retry ──────────────
    if let Some(slash_pos) = model.rfind('/') {
        let bare_query = &model[slash_pos + 1..];
        if !bare_query.is_empty() {
            if let Some(&entry) = db.exact.get(bare_query) {
                return Some(make_pricing(bare_query, entry));
            }
        }
    }

    // ── Tier 3: Exact match in stripped DB index ────────────────────────
    if let Some(&entry) = db.stripped.get(model) {
        return Some(make_pricing(model, entry));
    }

    // ── Tier 4: Prefix match (both directions, both maps) ──────────────
    let mut best_forward: Option<(&str, PricingEntry)> = None; // query starts_with key
    let mut best_reverse: Option<(&str, PricingEntry)> = None; // key starts_with query

    // Search exact map.
    for (key, &entry) in &db.exact {
        // Forward: query is more specific than key (e.g., "gpt-4o-2024-08-06" matches "gpt-4o")
        if model.starts_with(key.as_str()) {
            match &best_forward {
                Some((k, _)) if k.len() >= key.len() => {}
                _ => best_forward = Some((key.as_str(), entry)),
            }
        }
        // Reverse: query is less specific than key (e.g., "claude-3-5-sonnet" matches "claude-3-5-sonnet-20241022")
        // Only match if the next char after query in key is '-' or end (avoid "o3" matching "o3-mini")
        if key.len() > model.len() && key.starts_with(model) {
            let next_char = key.as_bytes()[model.len()];
            if next_char == b'-' || next_char == b'.' {
                match &best_reverse {
                    Some((k, _)) if k.len() <= key.len() => {}
                    _ => best_reverse = Some((key.as_str(), entry)),
                }
            }
        }
    }

    // Search stripped map (same logic).
    for (key, &entry) in &db.stripped {
        if model.starts_with(key.as_str()) {
            match &best_forward {
                Some((k, _)) if k.len() >= key.len() => {}
                _ => best_forward = Some((key.as_str(), entry)),
            }
        }
        if key.len() > model.len() && key.starts_with(model) {
            let next_char = key.as_bytes()[model.len()];
            if next_char == b'-' || next_char == b'.' {
                match &best_reverse {
                    Some((k, _)) if k.len() <= key.len() => {}
                    _ => best_reverse = Some((key.as_str(), entry)),
                }
            }
        }
    }

    // Prefer forward prefix (more specific query) over reverse (less specific query).
    if let Some((key, entry)) = best_forward {
        return Some(make_pricing(key, entry));
    }
    if let Some((key, entry)) = best_reverse {
        return Some(make_pricing(key, entry));
    }

    // ── Tier 5: Provider-based fallback ─────────────────────────────────
    provider_fallback(model)
}

/// Returns the total number of models in the embedded pricing database.
///
/// Useful for diagnostics and health checks.
pub fn model_count() -> usize {
    DB.exact.len()
}

/// Returns the total number of provider-stripped aliases in the database.
pub fn stripped_alias_count() -> usize {
    DB.stripped.len()
}

/// Fallback pricing based on provider name patterns in the model string.
///
/// Returns conservative (mid-range) estimates so that unknown model variants
/// still get a cost estimate rather than $0.
fn provider_fallback(model: &str) -> Option<ModelPricing> {
    let lower = model.to_lowercase();

    let (provider, input, output) = if lower.contains("gpt") {
        // Conservative mid-range OpenAI pricing
        ("openai", dec!(2.50), dec!(10.00))
    } else if lower.contains("claude") {
        ("anthropic", dec!(3.00), dec!(15.00))
    } else if lower.contains("gemini") {
        ("google", dec!(1.25), dec!(5.00))
    } else if lower.contains("mistral") || lower.contains("codestral") {
        ("mistral", dec!(0.50), dec!(1.50))
    } else if lower.contains("llama") {
        ("meta", dec!(0.60), dec!(0.60))
    } else if lower.contains("deepseek") {
        ("deepseek", dec!(0.55), dec!(2.19))
    } else if lower.contains("command") {
        ("cohere", dec!(0.15), dec!(0.60))
    } else {
        return None;
    };

    Some(ModelPricing {
        model: model.to_string(),
        provider: provider.to_string(),
        input_usd_per_1m: input,
        output_usd_per_1m: output,
    })
}

/// Legacy API compatibility: returns all entries from the database as a Vec.
///
/// Prefer `lookup_pricing()` for single-model lookups — this allocates.
pub fn default_pricing() -> Vec<ModelPricing> {
    DB.exact
        .iter()
        .map(|(name, &entry)| make_pricing(name, entry))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Original tests (all preserved) ──────────────────────────────────

    #[test]
    fn lookup_known_model() {
        let pricing = lookup_pricing("gpt-4o").expect("should find gpt-4o");
        assert_eq!(pricing.model, "gpt-4o");
        assert_eq!(pricing.provider, "openai");
        assert_eq!(pricing.input_usd_per_1m, dec!(2.50));
        assert_eq!(pricing.output_usd_per_1m, dec!(10.00));
    }

    #[test]
    fn lookup_totally_unknown() {
        assert!(lookup_pricing("totally-unknown-model").is_none());
    }

    #[test]
    fn lookup_prefix_match() {
        // LiteLLM has "gpt-4o-2024-08-06" as an exact entry with the same pricing.
        // With the large database, exact match takes priority over prefix match.
        let pricing = lookup_pricing("gpt-4o-2024-08-06").expect("should find gpt-4o variant");
        assert_eq!(pricing.input_usd_per_1m, dec!(2.50));
    }

    #[test]
    fn lookup_prefix_match_longest_wins() {
        // LiteLLM has "gpt-4o-mini-2024-07-18" as an exact entry.
        let pricing =
            lookup_pricing("gpt-4o-mini-2024-07-18").expect("should find gpt-4o-mini variant");
        assert_eq!(pricing.input_usd_per_1m, dec!(0.15));
    }

    #[test]
    fn lookup_anthropic_prefix_match() {
        let pricing = lookup_pricing("claude-3-5-sonnet-20241022")
            .expect("should prefix-match claude-3-5-sonnet");
        // LiteLLM has this as an exact match
        assert_eq!(pricing.provider, "anthropic");
        assert_eq!(pricing.input_usd_per_1m, dec!(3.00));
    }

    #[test]
    fn claude_sonnet_4_dated_variant() {
        let pricing = lookup_pricing("claude-sonnet-4-20250514")
            .expect("should prefix-match claude-sonnet-4");
        assert_eq!(pricing.input_usd_per_1m, dec!(3.00));
        assert_eq!(pricing.output_usd_per_1m, dec!(15.00));
    }

    #[test]
    fn claude_opus_4_5_not_legacy_opus_4() {
        // "claude-opus-4.5-20260101" MUST match "claude-opus-4.5" ($5/$25),
        // NOT the legacy "claude-opus-4" ($15/$75).
        let pricing = lookup_pricing("claude-opus-4.5-20260101")
            .expect("should prefix-match claude-opus-4.5");
        assert_eq!(pricing.model, "claude-opus-4.5");
        assert_eq!(pricing.input_usd_per_1m, dec!(5.00));
        assert_eq!(pricing.output_usd_per_1m, dec!(25.00));
    }

    #[test]
    fn gemini_prefix_match() {
        let pricing =
            lookup_pricing("gemini-2.5-pro-latest").expect("should prefix-match gemini-2.5-pro");
        assert_eq!(pricing.model, "gemini-2.5-pro");
        assert_eq!(pricing.input_usd_per_1m, dec!(1.25));
        assert_eq!(pricing.output_usd_per_1m, dec!(10.00));
    }

    #[test]
    fn new_openai_models() {
        let p = lookup_pricing("gpt-4.1").expect("gpt-4.1");
        assert_eq!(p.input_usd_per_1m, dec!(2.00));

        let p = lookup_pricing("gpt-4.1-mini-2026").expect("gpt-4.1-mini variant");
        assert_eq!(p.model, "gpt-4.1-mini");
        assert_eq!(p.input_usd_per_1m, dec!(0.40));

        let p = lookup_pricing("o3").expect("o3");
        assert_eq!(p.input_usd_per_1m, dec!(2.00));

        let p = lookup_pricing("o4-mini").expect("o4-mini");
        assert_eq!(p.input_usd_per_1m, dec!(1.10));
    }

    #[test]
    fn deepseek_models() {
        let p = lookup_pricing("deepseek-chat").expect("deepseek-chat");
        assert_eq!(p.input_usd_per_1m, dec!(0.28));

        // LiteLLM has updated DeepSeek Reasoner pricing ($0.28/$0.42 per 1M).
        let p = lookup_pricing("deepseek-reasoner").expect("deepseek-reasoner");
        assert_eq!(p.input_usd_per_1m, dec!(0.28));
    }

    #[test]
    fn llama_models() {
        let p = lookup_pricing("llama-3.1-405b").expect("llama-3.1-405b");
        assert_eq!(p.input_usd_per_1m, dec!(3.00));
    }

    #[test]
    fn provider_fallback_unknown_claude() {
        // Unknown claude variant → provider fallback with conservative anthropic pricing
        let pricing =
            lookup_pricing("some-unknown-claude-model").expect("should provider-fallback");
        assert_eq!(pricing.provider, "anthropic");
        assert_eq!(pricing.input_usd_per_1m, dec!(3.00));
    }

    #[test]
    fn provider_fallback_unknown_gpt() {
        let pricing = lookup_pricing("gpt-99-ultra").expect("should provider-fallback for gpt");
        assert_eq!(pricing.provider, "openai");
    }

    #[test]
    fn provider_fallback_unknown_gemini() {
        let pricing =
            lookup_pricing("gemini-99-ultra").expect("should provider-fallback for gemini");
        assert_eq!(pricing.provider, "google");
    }

    #[test]
    fn provider_fallback_returns_none_for_truly_unknown() {
        assert!(lookup_pricing("totally-unknown-model").is_none());
    }

    // ── New tests for LiteLLM embedded database ─────────────────────────

    #[test]
    fn database_loaded_with_many_models() {
        // The LiteLLM database should have 1800+ models.
        let count = model_count();
        assert!(
            count > 1800,
            "expected 1800+ models in pricing DB, got {count}"
        );
    }

    #[test]
    fn stripped_aliases_populated() {
        // Many models have provider prefixes, so stripped aliases should exist.
        let count = stripped_alias_count();
        assert!(count > 500, "expected 500+ stripped aliases, got {count}");
    }

    #[test]
    fn litellm_exact_match_claude_dated() {
        // LiteLLM has "claude-3-5-sonnet-20241022" as an exact entry.
        let p = lookup_pricing("claude-3-5-sonnet-20241022").expect("exact LiteLLM match");
        assert_eq!(p.input_usd_per_1m, dec!(3.00));
        assert_eq!(p.output_usd_per_1m, dec!(15.00));
    }

    #[test]
    fn litellm_provider_prefixed_model() {
        // "gemini/gemini-2.5-pro" should be found via exact match.
        let p = lookup_pricing("gemini/gemini-2.5-pro").expect("prefixed model");
        assert_eq!(p.input_usd_per_1m, dec!(1.25));
    }

    #[test]
    fn litellm_provider_prefix_stripped_from_query() {
        // If user passes "openai/gpt-4o", strip prefix and find "gpt-4o".
        let p = lookup_pricing("openai/gpt-4o").expect("stripped query");
        assert_eq!(p.input_usd_per_1m, dec!(2.50));
    }

    #[test]
    fn litellm_stripped_db_lookup() {
        // User queries "mistral-large-latest" without prefix.
        // DB has "mistral/mistral-large-latest" → stripped to "mistral-large-latest".
        let p = lookup_pricing("mistral-large-latest").expect("stripped DB lookup");
        assert!(p.input_usd_per_1m > Decimal::ZERO);
    }

    #[test]
    fn reverse_prefix_match_short_claude() {
        // "claude-3-5-sonnet" is shorter than "claude-3-5-sonnet-20241022" in the DB.
        // Reverse prefix: DB key starts_with query.
        let p = lookup_pricing("claude-3-5-sonnet").expect("reverse prefix match");
        assert_eq!(p.input_usd_per_1m, dec!(3.00));
        assert_eq!(p.provider, "anthropic");
    }

    #[test]
    fn litellm_deepseek_via_provider_prefix() {
        // "deepseek/deepseek-chat" should be found as exact match.
        let p = lookup_pricing("deepseek/deepseek-chat").expect("prefixed deepseek");
        assert_eq!(p.input_usd_per_1m, dec!(0.28));
    }

    #[test]
    fn estimate_cost_still_works() {
        // Verify the ModelPricing.estimate_cost method works with LiteLLM data.
        let p = lookup_pricing("gpt-4o").expect("gpt-4o");
        let cost = p.estimate_cost(1000, 500);
        // input: 2.50 * 1000/1_000_000 = 0.0025
        // output: 10.00 * 500/1_000_000 = 0.005
        assert_eq!(cost, dec!(0.0075));
    }

    #[test]
    fn many_providers_covered() {
        // Verify that diverse providers are represented.
        let test_cases = vec![
            ("gpt-4o", "openai"),
            ("claude-3-5-sonnet-20241022", "anthropic"),
            ("gemini-2.5-pro", "google"),
            ("deepseek-chat", "deepseek"),
            ("command-r-plus", "cohere"),
        ];
        for (model, expected_provider) in test_cases {
            let p = lookup_pricing(model).unwrap_or_else(|| panic!("should find {model}"));
            assert_eq!(
                p.provider, expected_provider,
                "model {model} should be provider {expected_provider}, got {}",
                p.provider
            );
        }
    }

    #[test]
    fn manual_override_takes_priority() {
        // "claude-opus-4.5" is a manual override (dot notation).
        // LiteLLM has "claude-opus-4-5" (dash notation) which is different.
        let p = lookup_pricing("claude-opus-4.5").expect("manual override");
        assert_eq!(p.input_usd_per_1m, dec!(5.00));
        assert_eq!(p.output_usd_per_1m, dec!(25.00));
    }

    #[test]
    fn manual_override_codestral() {
        let p = lookup_pricing("codestral").expect("codestral override");
        assert_eq!(p.input_usd_per_1m, dec!(0.30));
        assert_eq!(p.output_usd_per_1m, dec!(0.90));
    }

    #[test]
    fn default_pricing_returns_all() {
        let all = default_pricing();
        assert!(
            all.len() > 1800,
            "default_pricing() should return 1800+ entries, got {}",
            all.len()
        );
    }
}
