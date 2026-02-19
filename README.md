# Govrix Platform

> AI Agent Governance Platform — enforce policy, mask PII, track budgets

Govrix Platform is the commercial enforcement layer built on top of [Scout OSS](https://github.com/manaspros/govrix-scout). Where Scout observes, Govrix enforces: YAML-based policy rules, inline PII masking, budget limits, compliance tagging, and mTLS agent identity — all without touching your agents.

## Prerequisites

- Rust 1.85+
- PostgreSQL (optional — falls back to fail-open mode without it)
- SSH access to `github.com/manaspros/govrix-scout` (Scout is pulled as a git dependency)

## Quick Start

```bash
git clone git@github.com:manaspros/govrix.git
cd govrix

# Copy and edit the config
cp config/govrix.default.toml config/govrix.toml

# Set your license key (omit to run as Community tier)
export GOVRIX_LICENSE_KEY="<base64-encoded-license>"

# Point to your config file
export GOVRIX_CONFIG="config/govrix.toml"

# Optional: PostgreSQL connection string
export AGENTMESH_DATABASE_URL="postgres://govrix:govrix@localhost:5432/govrix"

# Build and run
cargo run --release -p govrix-server
```

The proxy listens on `:4000`, the management API on `:4001`.

Point your agents at the proxy:

```bash
export OPENAI_BASE_URL=http://localhost:4000/v1
export ANTHROPIC_BASE_URL=http://localhost:4000/anthropic
```

## License Tiers

| Tier | Max Agents | Retention | Policy | PII Masking | Compliance | mTLS |
|------|-----------|-----------|--------|-------------|------------|------|
| Community | 5 | 7d | No | No | No | No |
| Starter | 25 | 30d | Yes | No | No | No |
| Growth | 100 | 90d | Yes | Yes | Yes | No |
| Enterprise | Unlimited | 365d | Yes | Yes | Yes | Yes |

No license key supplied = Community tier. Expired or invalid keys also fall back to Community.

## API Endpoints

All Scout endpoints are available. Platform adds:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/platform/health` | Status, version, license tier, mTLS state |
| GET | `/api/v1/platform/license` | Tier, max agents, enabled features |
| GET | `/api/v1/policies` | Rule counts, policy/PII enabled flags |
| POST | `/api/v1/policies/reload` | Hot-reload YAML rules at runtime |
| GET | `/api/v1/tenants` | Tenant list with agent limits |

`POST /api/v1/policies/reload` accepts `{"rules_yaml": "..."}` or `{"rules_file": "/path/to/rules.yaml"}`.

## Architecture

Four crates:

```
govrix-common   — config loading, license validation, tenant types
govrix-policy   — YAML policy engine, PII masking, budget tracker, Scout hook bridge
govrix-identity — CA generation (rcgen), per-agent cert issuance, mTLS config
govrix-server   — startup orchestration, REST API, Scout proxy integration
```

`govrix-server` owns startup: validates the license, optionally generates a CA, initializes the policy engine and budget tracker, then hands a `GovrixPolicyHook` to Scout's proxy. The management API merges Scout's routes with the platform's own routes.

See [`docs/PLATFORM_ARCHITECTURE.md`](docs/PLATFORM_ARCHITECTURE.md) for the detailed architecture and [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for the development workflow.

## Built on Scout

Scout (govrix-scout) is the open-source transparent proxy that handles the actual HTTP proxying, event capture, and audit logging. Govrix Platform embeds Scout via `PolicyHook` and `serve_with_pool_and_routes()` — Scout remains entirely unmodified.

Scout OSS: https://github.com/manaspros/govrix-scout

## License

Copyright (c) 2026 Govrix. All rights reserved.

This software is proprietary and requires a commercial license for use. See your license agreement for terms.
