<p align="center">
  <img src="https://img.shields.io/badge/govrix-scout-AI%20Agent%20Governance-8B5CF6?style=for-the-badge&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0id2hpdGUiPjxwYXRoIGQ9Ik0xMiAyQzYuNDggMiAyIDYuNDggMiAxMnM0LjQ4IDEwIDEwIDEwIDEwLTQuNDggMTAtMTBTMTcuNTIgMiAxMiAyem0tMiAxNWwtNS01IDEuNDEtMS40MUwxMCAxNC4xN2w3LjU5LTcuNTlMMTkgOGwtOSA5eiIvPjwvc3ZnPg==&logoColor=white" alt="Govrix Scout" />
</p>

<h1 align="center">Govrix Scout</h1>

<p align="center">
  <b>Know what your AI agents are doing. Before your auditor asks.</b>
</p>

<p align="center">
  <a href="#-quick-start"><img src="https://img.shields.io/badge/Setup-2%20Minutes-00C853?style=flat-square" alt="Setup Time" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache%202.0-blue?style=flat-square" alt="License" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.82+-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust" /></a>
  <a href="https://react.dev"><img src="https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black" alt="React" /></a>
  <a href="https://www.timescale.com"><img src="https://img.shields.io/badge/TimescaleDB-16-FDB515?style=flat-square&logo=postgresql&logoColor=white" alt="TimescaleDB" /></a>
</p>

<br/>

<p align="center">
  <code>Wireshark</code> meets <code>Datadog</code> — purpose-built for AI agents.
</p>

---

<br/>

## The Problem

Your company is deploying AI agents. Fast. But nobody can answer these questions:

```
 "How many AI agents are running in production right now?"          → Nobody knows.
 "What data did the agent access before it sent that email?"        → No audit trail.
 "Why did our OpenAI bill spike to $47K last Tuesday?"              → Can't trace it.
 "Did any agent leak customer PII in the last 30 days?"            → Hope not.
 "Can we prove to auditors what our agents did and didn't do?"      → Definitely not.
```

**This isn't hypothetical.** This is happening right now at companies like yours:

- **Gartner predicts 40% of enterprise apps** will have AI agents by end of 2026 — up from <5% in 2025
- **Shadow AI breaches cost $670K more** than standard breaches (IBM 2025 Breach Report)
- **65% of shadow AI incidents** compromise customer PII — vs 53% for traditional breaches
- **69% of organizations** suspect employees are using unauthorized AI tools (Gartner 2025)
- **EU AI Act enforcement begins August 2026** — penalties up to **€35M or 7% of global revenue**

> Every AI agent action in an enterprise should be as auditable as a financial transaction. Today, that infrastructure does not exist.

**Govrix Scout makes it exist.**

<br/>

## What Govrix Scout Does

Govrix Scout is a **transparent proxy** that sits between your AI agents and their APIs. It captures every request and response — without touching a single line of your agent code.

```
  Your Agent                   Govrix Scout Proxy                    OpenAI / Anthropic
  ─────────                    ───────────────                    ──────────────────
       │                             │                                   │
       │  OPENAI_BASE_URL=           │                                   │
       │  localhost:4000/proxy/...   │                                   │
       │ ───────────────────────────>│                                   │
       │                             │  ┌─ Parse request ──────────┐    │
       │                             │  │  Extract: agent_id,      │    │
       │                             │  │  model, tokens, tools    │    │
       │                             │  │  Generate: session_id,   │    │
       │                             │  │  lineage_hash            │    │
       │                             │  └──────────────────────────┘    │
       │                             │                                   │
       │                             │  Fire event to channel (async)    │
       │                             │  ──────────────────────────────>  │
       │                             │  Forward request UNCHANGED ────>  │
       │                             │                                   │
       │                             │  <──────── Response ────────────  │
       │                             │                                   │
       │                             │  ┌─ Parse response ─────────┐    │
       │                             │  │  Extract: tokens, cost,  │    │
       │                             │  │  finish_reason, PII scan │    │
       │                             │  └──────────────────────────┘    │
       │                             │                                   │
       │  <────── Response UNCHANGED │                                   │
       │                             │                                   │
       │         Added latency: <5ms │                                   │
```

**One env var. Zero code changes. Full visibility.**

<br/>

## Why Govrix Scout — Not Just Another Logging Tool

| | Traditional Logging | APM Tools (Datadog, etc.) | **Govrix Scout** |
|---|---|---|---|
| **Understands AI protocols** | No | Partially | **Yes** — parses OpenAI, Anthropic, MCP, A2A natively |
| **Zero agent modification** | Requires SDK changes | Requires instrumentation | **One env var change** |
| **Agent auto-discovery** | Manual inventory | Manual tagging | **Automatic** — discovers agents from traffic |
| **Cost attribution** | DIY | Generic metrics | **Per-agent, per-model, per-request** |
| **PII detection** | Not built-in | Add-on | **Pattern flagging** — detects patterns; masking available with [Govrix Platform](https://Govrix Scout.io) |
| **Compliance-ready audit trail** | No | No | **Yes** — cryptographic lineage hash chain |
| **Latency overhead** | Varies | 10-50ms | **<5ms p99** |
| **Self-hosted** | Depends | SaaS-only | **100% self-hosted, your data stays yours** |

<br/>

## Quick Start

### Prerequisites

- Docker & Docker Compose v2

### 1. Clone and start

```bash
git clone https://github.com/manaspros/Govrix Scout.git
cd Govrix Scout
docker compose -f docker/docker-compose.yml up -d
```

### 2. Point your agents

Change one environment variable — that's it:

```bash
# OpenAI agents
export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1

# Anthropic agents
export ANTHROPIC_BASE_URL=http://localhost:4000/proxy/anthropic/v1
```

### 3. Open the dashboard

```
http://localhost:3000
```

### 4. Verify

```bash
curl http://localhost:4001/health
# {"status":"ok","version":"0.1.0"}
```

**That's it. Your agents are now observable.**

<br/>

## What You Get

### Real-Time Agent Inventory

Every agent routing through the proxy is automatically discovered and catalogued. No manual registration. No SDK. No config files per agent.

```
┌─────────────────────────────────────────────────────────────────┐
│  AGENTS                                              3 active   │
├──────────────────┬───────────┬────────┬──────────┬─────────────┤
│ Agent            │ Framework │ Status │ Requests │ Cost (24h)  │
├──────────────────┼───────────┼────────┼──────────┼─────────────┤
│ research-bot     │ LangChain │ ● active │  12,847 │    $142.30 │
│ code-assistant   │ CrewAI    │ ● active │   8,231 │     $89.44 │
│ support-agent    │ AutoGen   │ ● idle   │     342 │      $3.71 │
│ unknown-10.0.3.7 │ —         │ ⚠ unknown│      17 │      $0.89 │
└──────────────────┴───────────┴────────┴──────────┴─────────────┘
```

### Every Action, Logged

Every request and response is captured with full context — model, tokens, cost, latency, tool calls, and the complete payload for forensic replay.

```
┌─────────────────────────────────────────────────────────────────┐
│  EVENT TIMELINE                                    Feb 18, 2026 │
├──────────┬────────────────┬──────────┬────────┬───────┬────────┤
│ Time     │ Agent          │ Model    │ Tokens │ Cost  │ Status │
├──────────┼────────────────┼──────────┼────────┼───────┼────────┤
│ 14:03:22 │ research-bot   │ gpt-4o   │  2,847 │ $0.08 │ 200 ✓ │
│ 14:03:21 │ code-assistant │ claude-4 │  1,203 │ $0.04 │ 200 ✓ │
│ 14:03:19 │ research-bot   │ gpt-4o   │  4,102 │ $0.12 │ 200 ✓ │
│ 14:03:18 │ support-agent  │ gpt-4o-m │    891 │ $0.01 │ 200 ✓ │
│ 14:03:15 │ research-bot   │ gpt-4o   │  3,221 │ $0.09 │ ⚠ PII │
└──────────┴────────────────┴──────────┴────────┴───────┴────────┘
```

### Cost Tracking That Actually Works

Know exactly where your AI spend is going — by agent, by model, by day. No more surprise bills.

```
┌─────────────────────────────────────────────────────────────────┐
│  COST BREAKDOWN                          Last 7 days: $847.21  │
│                                                                 │
│  By Agent                    │  By Model                       │
│  ────────                    │  ────────                       │
│  research-bot    ████████ $412 │  gpt-4o       ████████ $523   │
│  code-assistant  █████   $267 │  claude-4     █████   $198    │
│  support-agent   ██     $089 │  gpt-4o-mini  ██     $089    │
│  data-pipeline   █      $079 │  gpt-3.5      █      $037    │
└─────────────────────────────────────────────────────────────────┘
```

### PII Pattern Flagging

Govrix Scout flags sensitive data patterns in request and response payloads, logging their type and location so you know when PII flows through your agent traffic.

- **Email addresses** — `john.doe@company.com` → flagged in compliance tag
- **Phone numbers** — US format patterns
- **Social Security Numbers** — `XXX-XX-XXXX` patterns
- **Credit card numbers** — Major card patterns (Luhn-eligible)
- **IP addresses** — Internal network addresses in prompts

> Govrix Scout **detects and flags** PII patterns — it does not store the actual values. For **real-time PII masking and blocking** (redacting sensitive data before it reaches the upstream API), see [Govrix Platform](https://Govrix Scout.io).

### Tamper-Evident Audit Trail

Every event carries four mandatory compliance fields — no exceptions, no configuration needed:

| Field | Purpose |
|-------|---------|
| `session_id` | Groups related requests into agent conversations |
| `timestamp` | UTC ISO-8601, microsecond precision |
| `lineage_hash` | SHA-256 Merkle chain — proves event ordering and integrity |
| `compliance_tag` | Policy evaluation result: `pass:all`, `warn:pii_email`, `block:budget` |

This gives you a **cryptographically linked chain of evidence** showing exactly what each agent did, when, and whether it triggered any policy violations.

<br/>

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                     YOUR INFRASTRUCTURE                          │
│                                                                  │
│   ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐              │
│   │LangChain│ │ CrewAI  │ │ AutoGen │ │ Custom  │  Any agent.  │
│   │  Agent  │ │  Agent  │ │  Agent  │ │  Agent  │  Any framework│
│   └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘              │
│        └───────────┴─────┬─────┴───────────┘                    │
│                          │                                       │
│              Change one env var:                                 │
│              OPENAI_BASE_URL=localhost:4000/proxy/openai/v1      │
│                          │                                       │
│                ┌─────────▼──────────┐                            │
│                │                    │    Rust / hyper + tokio     │
│                │   Govrix Scout PROXY  │    <1ms p50 / <5ms p99     │
│                │      :4000         │    Fail-open design        │
│                │                    │                            │
│                └──┬──────────────┬──┘                            │
│                   │              │                                │
│         ┌─────────▼───┐  ┌──────▼──────────┐                    │
│         │  Upstream    │  │ Async Event     │                    │
│         │  APIs        │  │ Pipeline        │                    │
│         │              │  │                 │                    │
│         │ • OpenAI     │  │ Channel (10K)   │                    │
│         │ • Anthropic  │  │    ↓            │                    │
│         │ • MCP        │  │ Batch Writer    │                    │
│         │ • A2A        │  │ (100ms/100 evt) │                    │
│         └──────────────┘  └──────┬──────────┘                    │
│                                  │                                │
│                        ┌─────────▼──────────┐                    │
│                        │   TimescaleDB      │                    │
│                        │   :5432            │                    │
│                        │                    │                    │
│                        │  • events table    │  7-day retention   │
│                        │  • agents table    │  Auto-compression  │
│                        │  • cost_daily view │  Hypertable        │
│                        └─────────┬──────────┘                    │
│                                  │                                │
│                        ┌─────────▼──────────┐                    │
│                        │   REST API (axum)  │                    │
│                        │   :4001            │                    │
│                        │                    │                    │
│                        │  17 endpoints      │                    │
│                        │  Bearer auth       │                    │
│                        │  Prometheus        │                    │
│                        └─────────┬──────────┘                    │
│                                  │                                │
│                        ┌─────────▼──────────┐                    │
│                        │   Dashboard        │                    │
│                        │   :3000            │                    │
│                        │                    │                    │
│                        │  React 18          │                    │
│                        │  Real-time refresh │                    │
│                        │  Dark theme        │                    │
│                        └────────────────────┘                    │
└──────────────────────────────────────────────────────────────────┘
```

### Why Rust for the Proxy?

The proxy is the **hot path** — every single AI request flows through it. We use Rust with `hyper` directly (not a framework) to guarantee:

- **<1ms p50 latency** — your agents won't notice it's there
- **<5ms p99 latency** — even under load
- **Zero garbage collection pauses** — predictable performance
- **Fail-open design** — if Govrix Scout has an internal error, your agent traffic continues uninterrupted

The database write is **fire-and-forget** — sent to a bounded async channel, never awaited in the request path.

<br/>

## Supported Protocols

| Protocol | Route | Status |
|----------|-------|--------|
| **OpenAI API** | `/proxy/openai/v1/*` | Full support — chat, completions, embeddings, streaming |
| **Anthropic API** | `/proxy/anthropic/v1/*` | Full support — messages, streaming |
| **MCP** (Model Context Protocol) | `/proxy/mcp/{server}/*` | Passthrough (structured parsing planned) |
| **A2A** (Agent-to-Agent) | `/proxy/a2a/{agent}/*` | Passthrough (structured parsing planned) |
| **Custom upstream** | `/proxy/custom/{name}/*` | Generic passthrough for any HTTP API |

<br/>

## API Reference

All endpoints on port `4001`. Responses follow `{"data": [...], "total": N}` for lists.

<details>
<summary><b>Health & Monitoring</b></summary>

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Liveness check |
| `GET` | `/ready` | Readiness — verifies DB connection |
| `GET` | `/metrics` | Prometheus metrics |

</details>

<details>
<summary><b>Events</b></summary>

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/events` | List & filter events. Params: `agent_id`, `kind`, `protocol`, `model`, `from`, `to`, `search`, `min_cost`, `limit`, `offset`, `sort` |
| `GET` | `/api/v1/events/:id` | Single event with full payload |
| `GET` | `/api/v1/events/sessions/:session_id` | All events in a session (chronological) |
| `GET` | `/api/v1/events/stream` | SSE real-time event stream |

</details>

<details>
<summary><b>Agents</b></summary>

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/agents` | List agents. Params: `status`, `label`, `sort`, `limit`, `offset` |
| `GET` | `/api/v1/agents/:id` | Agent detail with aggregated stats |
| `PUT` | `/api/v1/agents/:id` | Update name, description, labels |
| `POST` | `/api/v1/agents/:id/retire` | Retire an agent |
| `GET` | `/api/v1/agents/:id/events` | Events for a specific agent |

</details>

<details>
<summary><b>Costs</b></summary>

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/costs/summary` | Time-bucketed costs. Params: `from`, `to`, `granularity` |
| `GET` | `/api/v1/costs/breakdown` | Breakdown by `group_by`: agent, model, protocol |

</details>

<details>
<summary><b>Reports & Config</b></summary>

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/v1/reports/types` | Available report types |
| `POST` | `/api/v1/reports/generate` | Generate a report (202 Accepted) |
| `GET` | `/api/v1/reports` | List generated reports |
| `GET` | `/api/v1/config` | Current config (secrets redacted) |

</details>

<br/>

## Configuration

Default config at `config/Govrix Scout.default.toml`. Override anything with environment variables:

```bash
# Database
GOVRIX_STORE__DATABASE_URL=postgresql://user:pass@host:5432/Govrix Scout
# Proxy listens here
GOVRIX_PROXY__LISTEN_PORT=4000
# Management API uses Bearer auth under /api/v1/*
GOVRIX_API_KEY=amesh_live_your_secret_key_here
# Filter telemetry output
RUST_LOG=Govrix Scout=info
```

<details>
<summary><b>Full TOML config reference</b></summary>

```toml
[proxy]
listen_address = "0.0.0.0"
listen_port = 4000
max_connections = 10000
request_timeout_seconds = 300

[proxy.upstreams.openai]
base_url = "https://api.openai.com"
timeout_seconds = 120

[proxy.upstreams.anthropic]
base_url = "https://api.anthropic.com"
timeout_seconds = 120

[store]
database_url = "postgresql://Govrix Scout:Govrix Scout@localhost:5432/Govrix Scout"
max_connections = 20
retention_days = 7
batch_size = 100
batch_interval_ms = 100
channel_capacity = 10000

[pricing.openai]
"gpt-4o".input_per_1m = 2.50
"gpt-4o".output_per_1m = 10.00

[pricing.anthropic]
"claude-sonnet-4-20250514".input_per_1m = 3.00
"claude-sonnet-4-20250514".output_per_1m = 15.00
```

</details>

<br/>

## Development

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | 1.82+ | [rustup.rs](https://rustup.rs) |
| Node.js | 20+ | [nodejs.org](https://nodejs.org) |
| pnpm | 9+ | `corepack enable` |
| Docker | Latest | [docker.com](https://docker.com) |

### First-time setup

```bash
git clone https://github.com/manaspros/Govrix Scout.git
cd Govrix Scout
./scripts/setup.sh
```

### Development workflow

```bash
# Terminal 1: Start database
make docker-up

# Terminal 2: Start proxy + API
make dev-proxy

# Terminal 3: Start dashboard (hot reload)
make dev-dashboard

# Seed demo data
./scripts/seed-demo-data.sh
```

### Testing

```bash
make test        # 156 Rust unit tests
make lint        # cargo clippy -- -D warnings
make fmt-check   # cargo fmt -- --check
```

### All make targets

```bash
make help        # List everything
make setup       # First-time setup
make build       # Release build
make test        # Run all tests
make lint        # Clippy
make fmt         # Format code
make docker-up   # Start containers
make docker-down # Stop containers
make migrate     # Run SQL migrations
make clean       # Remove artifacts
```

<br/>

## The Regulatory Clock Is Ticking

<table>
<tr>
<td width="50%">

### EU AI Act Timeline
- **Feb 2025**: AI literacy obligations began
- **Aug 2025**: Prohibited AI practices enforced
- **Aug 2026**: **Full enforcement** — high-risk AI systems must comply
- **Penalties**: Up to **€35M** or **7% of global revenue**

### What Auditors Are Asking
1. How many AI agents do you have?
2. What data do they access?
3. Can you prove what they did last Tuesday?
4. Do you detect PII in agent traffic?
5. Is there a tamper-evident audit trail?

</td>
<td width="50%">

### Govrix Scout Answers All Five

```
✓ Agent auto-discovery
  → Full inventory, always current

✓ Request/response logging
  → Every API call, every payload

✓ Tamper-evident lineage chain
  → SHA-256 Merkle hash per event

✓ PII pattern flagging
  → 5 pattern types, zero PII storage
  → Masking/blocking via Govrix Platform

✓ Session-grouped audit trail
  → Reconstruct any agent conversation
```

</td>
</tr>
</table>

<br/>

## Govrix Scout Enterprise

The open-source core gives you full visibility. When you need **control**, Govrix Scout Enterprise adds:

| Capability | What It Does |
|------------|-------------|
| **Real-time policy engine** | Block PII before it reaches the API, enforce token budgets, require human approval for high-risk actions |
| **Session recorder** | Cryptographically signed replay of every agent session — evidence-grade for legal and compliance |
| **Compliance templates** | One-click reports for SOC 2, EU AI Act, HIPAA, FINRA — mapped to your actual agent data |
| **A2A identity layer** | Agent certificates, capability attestation, permission scoping for multi-agent systems |
| **SSO + RBAC** | Okta, Azure AD, Google Workspace. Role-based access to dashboard and API |
| **Unlimited scale** | No agent cap, no retention limit, multi-cluster federation, Kubernetes Helm charts |

<p align="center">
  <a href="https://Govrix Scout.io"><b>Learn more at Govrix Scout.io</b></a>
  &nbsp;·&nbsp;
  <a href="mailto:hello@Govrix Scout.io">Contact sales</a>
</p>

<br/>

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Development environment setup
- Code standards (Rust: `clippy` + `rustfmt`, TypeScript: ESLint + Prettier)
- Conventional commit format
- PR process and review guidelines

<br/>

## License

[Apache 2.0](LICENSE) — free to use, modify, and distribute. Forever.

---

<p align="center">
  <sub>Built for the teams that deploy AI agents — and the compliance teams that have to answer for them.</sub>
</p>
