# LLM Pricing Updater

## Overview

The pricing updater is a critical infrastructure component that automatically fetches and maintains current token costs for Large Language Models (LLMs) from multiple providers. This solves a fundamental problem: **LLM pricing changes frequently and unexpectedly**, but the Govrix Scout system needs accurate costs for:

1. **Cost Attribution Dashboard** — Shows users accurate USD spend per organization/project/model
2. **Budget Enforcement (F33: Denial-of-Wallet)** — Prevents overspend by throttling requests when approaching budget limits
3. **Cost-First Model Selection (F30)** — Routes requests to cheaper models that meet latency/quality requirements

Without this updater, hard-coded pricing in the Rust binary becomes stale within weeks, leading to:
- **Silent cost underestimation** on dashboards (showing $5 spend when it's actually $12)
- **Budget alerts firing at wrong thresholds** (refusing $50 of work when budget allowed $60)
- **Inability to react** to price cuts like Anthropic's 67% Opus reduction (Oct 2025)

## Architecture

### Data Flow

```
┌──────────────────────────────────────────────────────────────┐
│                  update_pricing.py (main script)              │
└──────────────────┬───────────────────────────────────────────┘
                   │
        ┌──────────┴──────────┐
        │                     │
┌───────▼──────────────┐  ┌──▼─────────────────────┐
│ OpenAIFetcher        │  │ AnthropicFetcher       │
│ ├─ Live scrape       │  │ ├─ Live scrape        │
│ │  (openai.com)      │  │ │  (claude platform)   │
│ └─ Fallback table    │  │ └─ Fallback table      │
└───────┬──────────────┘  └──┬─────────────────────┘
        │                     │
        └──────────┬──────────┘
                   │
        ┌──────────▼─────────┐
        │ Validation         │
        │ • Min 5 models     │
        │ • Prices > 0       │
        │ • Output >= Input  │
        └──────────┬─────────┘
                   │
        ┌──────────▼────────────────┐
        │ Diff Detection            │
        │ • Float epsilon: 0.001    │
        │ • % change calc           │
        │ • Human-readable report   │
        └──────────┬────────────────┘
                   │
                   ▼
        config/pricing.json
        ├─ schema_version: "1"
        ├─ last_updated: ISO8601
        ├─ models_count: NNN
        └─ models: {...}
```

### Fallback-First Design

To ensure reliability even when scraping fails:

1. **Live Scrape First** — Attempts to fetch current prices from OpenAI/Anthropic websites
2. **Fallback to Embedded Table** — If scraping fails (network error, page changed, etc.), uses a hard-coded pricing table that's kept current
3. **Validation Gate** — Rejects data with 0 or negative prices, or suspiciously inverted output<input ratios

This means the script **always succeeds** — worst case, you get last-known-good prices from the fallback table.

### Fetcher Pattern

Each provider has its own fetcher class implementing `BaseFetcher`:

```python
class BaseFetcher:
    def fetch(self) -> list[ModelPricing]:
        """Return list of current prices for this provider."""
        
    def _scrape_pricing_page(self) -> Optional[list[ModelPricing]]:
        """Live scrape via requests + BeautifulSoup."""
        
    def _build_from_fallback(self) -> list[ModelPricing]:
        """Parse hard-coded fallback table."""
```

This design makes it easy to add support for Bedrock, Azure OpenAI, Vertex AI, etc. in Phase 2-A (multi-provider routing).

## Usage

### Installation

```bash
# From repo root
pip install -r scripts/pricing/requirements.txt
```

### Commands

#### 1. Update Pricing (Normal Operation)

```bash
# One-liner
make update-pricing

# Or manually
python scripts/pricing/update_pricing.py
```

**Output:**
- Fetches live prices from OpenAI + Anthropic
- Validates data
- Compares against current `config/pricing.json`
- Writes updated JSON if changes detected
- Prints human-readable diff:

```
──────────────────────────────────────────────────────────────
  🔔  PRICING CHANGES DETECTED  (2 model(s))
──────────────────────────────────────────────────────────────
  📊  CHANGED  gpt-4o                             in: $2.50 → $5.00 (↑100.0%)  out: $10.00 → $15.00 (↑50.0%)
  ➕  ADDED    claude-opus-4-6                    in=$5.00  out=$25.00
──────────────────────────────────────────────────────────────

✅ Wrote 25 models -> config/pricing.json
```

#### 2. Preview Changes (Dry Run)

```bash
# See what would change without writing
make preview-pricing

# Or manually
python scripts/pricing/update_pricing.py --dry-run
```

**Output:** Shows diff but does NOT write to `config/pricing.json`.

#### 3. Check Staleness (CI Mode)

```bash
# Exit with code 1 if outdated (for CI/CD pipelines)
make check-pricing

# Or manually
python scripts/pricing/update_pricing.py --check
```

**Use in CI:** Add to GitHub Actions to fail builds if pricing is >30 days old.

## Integration with Govrix Scout Proxy

### Current Status ⚠️

The script generates `config/pricing.json` with valid schema, but the Rust proxy is **not yet reading it**. This is a staged rollout:

**Phase 1 (Current):** Generate accurate pricing JSON ✅
**Phase 2 (F33/F30):** Update `crates/govrix-scout-proxy/src/costs.rs` to:
- Read `config/pricing.json` at startup
- Fall back to compiled-in defaults if file missing
- Enable hot-reload for pricing updates without recompile

### Timeline

- **Now (Phase 1):** Script runs daily via CI cron, keeps `config/pricing.json` fresh
- **Phase 3 (Q2 2026):** Proxy reads pricing JSON
  - F30: Cost-first model cascade uses actual prices
  - F33: Denial-of-Wallet enforces accurate USD/hour budgets

## Configuration

### `pricing.json` Schema

```json
{
  "schema_version": "1",
  "last_updated": "2026-03-07T14:23:45.123456+00:00",
  "models_count": 25,
  "models": {
    "gpt-4o": {
      "provider": "openai",
      "input_per_1m_usd": 5.00,
      "output_per_1m_usd": 15.00,
      "context_window": 128000,
      "notes": null
    },
    "claude-opus-4-6": {
      "provider": "anthropic",
      "input_per_1m_usd": 3.00,
      "output_per_1m_usd": 15.00,
      "context_window": 200000,
      "notes": null
    },
    ...
  }
}
```

### Fallback Tables

Each fetcher includes a hard-coded `PROVIDER_FALLBACK` table with last-known-good prices:

- **OpenAI:** `fetchers/openai_fetcher.py` (GPT-4o, 4, 3.5-turbo, etc)
- **Anthropic:** `fetchers/anthropic_fetcher.py` (Claude 4.6, 4.5, 3.5, Opus, etc)

To update fallback tables when pricing changes:

```bash
# Edit the table in the fetcher file
vim scripts/pricing/fetchers/anthropic_fetcher.py

# Verify it still works
python scripts/pricing/update_pricing.py --dry-run
```

## Testing

Run the unit test suite:

```bash
# Run all tests
make test-pricing

# Or manually
pytest scripts/pricing/tests/ -v
```

**Coverage:**
- `test_diff_detects_*` — Diff detection (added, removed, changed, rounding)
- `test_validation_*` — Data validation (zero prices, inverted ratios, count)
- `test_fallback_returns_valid_models` — Fetcher fallback paths

## Troubleshooting

### "No pricing data fetched from any provider"

**Cause:** Both live scrapes and fallback tables failed.

**Fix:**
1. Check network: `curl https://openai.com/api/auth/session`
2. Check fallback tables are not empty
3. Verify BeautifulSoup can parse HTML (provider site may have changed structure)

### "Pricing is <= 0" validation error

**Cause:** Scraper parsed a non-numeric value (e.g., "Contact sales").

**Fix:**
1. Check the provider's website — they may list some models as "custom pricing"
2. Update `_scrape_pricing_page()` to skip non-standard models
3. Or add them to the fallback table manually

### "Too few models in result: 2 (expected at least 5)"

**Cause:** Scraper only found 2 models, likely due to HTML structure change.

**Fix:**
1. Run with `--dry-run` to see what was parsed
2. Add debug logging: `logger.debug(f"Parsed model: {model_name} {input_price}/{output_price}")`
3. Check if provider's pricing page changed
4. Fallback table will be used, but verify those prices are correct

## Maintenance

### Daily / Weekly

```bash
# Run as part of CI (e.g., GitHub Actions)
make check-pricing

# If it detects changes, PR is created with updated pricing.json
```

### Monthly

```bash
# Verify fallback tables are still accurate
# Compare against provider websites manually
# Update if there are major changes:

git diff config/pricing.json
git logstat scripts/pricing/fetchers/*.py
```

### When Adding New Providers (Phase 2-A)

1. Create `scripts/pricing/fetchers/provider_fetcher.py`
2. Extend `BaseFetcher`
3. Implement `fetch()`, `_scrape_pricing_page()`, `_build_from_fallback()`
4. Add to fetchers list in `update_pricing.py`
5. Add unit tests in `tests/test_pricing.py`
6. Update this README

## Dependencies

- `requests==2.32.3` — HTTP library
- `beautifulsoup4==4.12.3` — HTML parsing
- `lxml==5.3.0` — Fast XML/HTML parser (BeautifulSoup backend)
- `pytest==8.3.3` — Test runner
- `pytest-mock==3.14.0` — Pytest mocking plugin

## References

- **Phase 3 Design:** [docs/plans/2026-03-06-phase3-intelligent-proxy-design.md](../../docs/plans/)
- **Makefile targets:** [Makefile](../../Makefile) — `update-pricing`, `check-pricing`, `preview-pricing`
- **Config:** [config/pricing.json](../../config/pricing.json) — Generated output file
- **Proxy integration (TODO):** [crates/govrix-scout-proxy/src/costs.rs](../../crates/govrix-scout-proxy/src/costs.rs)

## Contributing

### Bug Reports

If pricing seems wrong:
1. Run `python scripts/pricing/update_pricing.py --dry-run`
2. Compare output against provider websites
3. File issue with: provider, model name, expected vs actual price

### Improvements

PRs welcome for:
- New provider fetchers
- Better HTML parsing (if provider changes structure)
- Caching to avoid repeated fetches
- Local pricing override file for air-gapped setups
- Prometheus metrics export (fetch duration, validation errors, etc)

---

**Last Updated:** 2026-03-07 by Govrix Team
