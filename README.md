<p align="center">
  <img src="docs/assets/logo.png" alt="Govrix Scout Logo" width="200" />
</p>

<h1 align="center">Govrix Scout</h1>

<p align="center">
  <b>Know what your AI agents are doing before your auditor asks.</b><br/>
  An open-source reverse proxy that logs, costs, and audits every LLM call your agents make. You change one base URL.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache%202.0-blue?style=flat-square" alt="License" /></a>
  <img src="https://img.shields.io/badge/Rust-1.75+-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black" alt="React" />
  <img src="https://img.shields.io/badge/TimescaleDB-PG16-FDB515?style=flat-square&logo=postgresql&logoColor=white" alt="TimescaleDB" />
</p>

<p align="center">
  <a href="https://govrix-scout-demo.vercel.app"><b>Live demo</b></a> ·
  <a href="https://govrix.dev"><b>Website</b></a> ·
  <a href="#7-run-locally"><b>Run locally</b></a> ·
  <a href="https://github.com/manaspros/govrix-scout/issues"><b>Issues</b></a>
</p>

---

## 1. Live demo

**https://govrix-scout-demo.vercel.app** is the real dashboard (19 pages) running against sample data.

> **Demo mode.** On Vercel the Rust proxy and TimescaleDB are *not* running. The dashboard talks to a read-only TypeScript port of the API (`api/index.ts`) that serves about 1,400 seeded events from 8 fictional agents. The data is generated in memory with the same schema as the migrations and the same SHA-256 lineage-hash algorithm as the real proxy. Edits you make in the UI (budgets, kill switch, acknowledge alert) are not persisted. Details in [section 6](#6-prototype-vs-full-deployment).
>
> Where to click: **Overview**, then **Traces**, **Risk Overview**, **Compliance**, **PII Activity**, **Anomalies**. The **Sessions** page wants a session ID: get one from `/api/v1/demo` (`sample_session_id`).

---

## 2. The problem

Teams are putting LLM agents into production faster than they can answer basic questions about them:

- **Which agents exist?** Someone's script has been calling GPT-4 for three weeks. Nobody registered it.
- **What did it send, and to whom?** Prompts carry emails, phone numbers, card numbers. Once they reach a third-party API you cannot take them back.
- **What did it cost?** The provider invoice is one number per key. It does not say which agent, session, or prompt burned the budget.
- **Can we prove it?** Auditors (SOC 2, HIPAA, EU AI Act Art. 12 record-keeping) ask for an event log you can show was not edited afterwards.

Who has this: platform, security, and compliance teams running more than a couple of agents, and anyone who has been surprised by an LLM bill.

Why the usual options fall short:

| Option | Gap |
|--------|-----|
| Provider dashboards | One key is one number. No per-agent attribution, no PII view, no cross-provider view. |
| SDK-based tracing (instrument each app) | You must touch every agent's code and keep it current. Agents you do not own stay invisible. |
| General API gateways | Built for routing and rate limits, not for per-agent audit, PII flags, or evidence an auditor can use. |
| Hosted observability SaaS | Your prompts and responses leave your infrastructure. |

---

## 3. Why Govrix Scout is good

**It sits on the wire, not in your code.** Point `OPENAI_BASE_URL` at the proxy and every call is captured. Agents are discovered from traffic. There is nothing to install in the agent.

What it does, and where to check it:

| Capability | Evidence in this repo |
|------------|-----------------------|
| One-line drop-in for OpenAI, Anthropic, MCP, A2A | Path routing for `/proxy/openai/`, `/proxy/anthropic/`, `/proxy/mcp/`, `/proxy/a2a/` in `crates/govrix-scout-proxy/src/proxy/handler.rs` |
| Hot path on raw `hyper`, not a web framework | `hyper::server::conn::http1` in `proxy/mod.rs`; the management API is a separate `axum` server on its own port |
| Logging never blocks agent traffic | Events go to a bounded `mpsc` channel via `try_send`. If it is full, the event is dropped and counted; the request still flows (`events.rs`) |
| Batched writes off the request path | 4 writer tasks, batches of 500 rows or every 50 ms (`WriterConfig::default`) |
| Tamper-evident ordering | Each event stores `sha256(prev_hash \| event_id \| agent_id \| timestamp_ms)`, starting from `GENESIS` per session (`compute_lineage_hash`) |
| PII flags: email, phone, SSN, credit card, IP | `crates/govrix-policy/src/pii.rs` |
| Cost per agent, model, and time | Token counts times a price table (`govrix-scout-common/src/models/pricing.rs`, `litellm_prices.json`) |
| Anomaly alerts | 8 detectors: cost, latency, exfil, security, fanout, error-rate, behavioral, legacy (`proxy/src/anomaly/`) |
| Time-series storage with compression and retention | Hypertable, compression, and retention policies in `init/004_create_hypertables.sql` |
| Budgets, sessions, traces, kill switch, compliance views | Migrations `006` to `014`, 19 dashboard pages |

**Claimed, not yet benchmarked:** "under 5 ms p99 added latency" (stated in `CLAUDE.md`, `CHANGELOG.md`, `CONTRIBUTING.md`). There is no benchmark harness in the repo yet. Treat it as a design target.

**Be precise about the audit trail:** it is a linear hash chain per session (edits and deletions inside a session break every later hash), not a Merkle tree. The repo does not yet ship a verifier command, and chain state lives in proxy memory, so a proxy restart starts a new `GENESIS` per session.

---

## 4. How it works

```mermaid
flowchart LR
    Agent["AI agents<br/>(any framework)"] -->|"OPENAI_BASE_URL=:4000/proxy/openai/v1"| Proxy
    subgraph Scout ["Govrix Scout (Rust)"]
        Proxy["Proxy :4000<br/>hyper hot path"]
        Chan["bounded mpsc channel<br/>(100k, try_send)"]
        Writers["4 batch writers<br/>(500 rows / 50 ms)"]
        API["Management API :4001<br/>axum"]
        Proxy -->|"fire and forget"| Chan --> Writers
    end
    Proxy <-->|"forwarded unchanged"| LLM["OpenAI / Anthropic / MCP"]
    Writers --> DB[("TimescaleDB<br/>events, agents, costs...")]
    API --> DB
    UI["Dashboard :3000<br/>React + Vite"] --> API
```

1. The agent calls the proxy instead of the provider. The proxy forwards the request and streams the response back unchanged, including SSE.
2. A copy of the exchange becomes an event carrying `session_id`, `timestamp`, `lineage_hash`, and `compliance_tag`. It is pushed onto a channel and the request path moves on.
3. Writer tasks batch events into TimescaleDB and upsert the agent registry. Anomaly checks run after the flush, off the hot path.
4. The dashboard reads from the management API.

If the database is down or the channel is full, traffic keeps flowing and events are dropped (counted in metrics). That is the fail-open design.

---

## 5. Engineering tradeoffs

| Decision | Chosen | Alternative | Why | Cost accepted |
|----------|--------|-------------|-----|---------------|
| Hot-path language and server | Rust with raw `hyper` | Go or Node gateway; or axum for everything | No GC pauses, so tail latency is predictable. Skips router and extractor overhead on the path every request takes (`CLAUDE.md` invariants) | Slower to change than a scripting gateway; needs the Rust toolchain and a C/C++ linker to build. The p99 target is not benchmarked yet |
| Persisting events | Bounded `mpsc` channel plus batch writer tasks | Synchronous DB insert per request | A slow or dead database must not add latency or errors to agent calls. Batches of 500 cut insert overhead (`events.rs`) | **Events can be dropped** when the 100k channel is full. Fine for observability; not a lossless audit guarantee |
| Integration method | Change one base URL (`OPENAI_BASE_URL`) | Per-language SDK or manual instrumentation | Works for agents you do not own and any framework that honours a base URL; no agent code to maintain | Only sees traffic routed through it; an agent can bypass it by calling the provider directly |
| Failure mode | Fail-open (`GOVRIX_PROXY_FAIL_OPEN`) | Fail-closed: block when logging is down | Observability must never take production down | During an outage you lose visibility and enforcement, not availability |
| Storage | TimescaleDB (hypertables, compression, retention) | Plain Postgres; ClickHouse or Kafka | Events are append-only time series, and compression and retention are one-liners while it stays plain SQL. ClickHouse is the planned tier 3 for scale (`CLAUDE.md` scale tiers) | Needs the Timescale extension. Managed Postgres that lacks it, or ships only the Apache-licensed subset, cannot run all migrations |
| Tamper evidence | SHA-256 hash chain per session | Merkle tree with signed roots; external ledger | Cheap to compute on write; any in-session edit or deletion breaks the later hashes | Chain state is in memory (restart means new genesis); no verifier yet; not signed or externally anchored |
| Hosting model | Self-hosted binary plus DB | SaaS | Prompts and responses never leave your infrastructure | You run and patch it. Report persistence and PDF export are stubbed in OSS |

---

## 6. Prototype vs full deployment

| | Live demo on Vercel | Full deployment (`docker compose`) |
|---|---|---|
| Dashboard | **Real.** The same React app from `dashboard/`, built and served as static files | Same |
| API | **Ported.** `api/index.ts` (a Vercel Function on Node.js) re-implements the read endpoints the dashboard calls, with the same JSON shapes as the Rust handlers | **Real.** Rust `axum` API on :4001 |
| Data | **Seeded.** About 1,400 events, 8 agents, 160 sessions/traces, 24 alerts, generated in memory from the repo's schema. The hash chain uses the real algorithm | **Real.** Written by the proxy into TimescaleDB |
| Proxy (:4000) | **Not running** | **Real** |
| Database | **None** | TimescaleDB (PostgreSQL 16) |
| Writes (kill switch, budgets, acknowledge) | Accepted, kept in function memory only, reset hourly | Persisted |
| Reports | Stubbed (also stubbed in the OSS API) | Stubbed in OSS |

**Why not run the real Rust API on Vercel?** Vercel has a Rust runtime (beta) and container-image functions. But the API is a thin layer over TimescaleDB: it needs a live database with hypertables, compression, and continuous aggregates (`init/014_continuous_aggregates.sql`). Neon, the Postgres on the Vercel Marketplace, supports only the Apache-2 subset of TimescaleDB (no compression, no continuous aggregates). The proxy also needs a long-lived process and an upstream LLM key. None of that fits a stateless function, so the lowest-risk path to a clickable demo is a read-only TypeScript port over seeded data. Run it locally to see the real thing.

Demo internals: `api/index.ts` (router), `api/_lib/seed.ts` (generator), `vercel.json` (rewrites `/api/*`, `/health`, `/ready`, `/metrics` to the function), `.vercelignore`. One dashboard fix came with it: `EventsPage` built a fresh `since` timestamp on every render, which changed the query key each time and refetched forever; it is now memoised.

---

## 7. Run locally

Requires [Docker](https://docker.com) with Compose v2.

```bash
git clone https://github.com/manaspros/govrix-scout.git
cd govrix-scout
docker compose -f docker/docker-compose.yml up -d
curl http://localhost:4001/health      # {"status":"ok",...}
# Dashboard: http://localhost:3000
```

One-line installers: `curl -sSL https://govrix.dev/install.sh | bash` (Linux/macOS), or on Windows `iwr -useb https://raw.githubusercontent.com/manaspros/govrix-scout/main/install.ps1 | iex`.

| Service | Port | Purpose |
|---------|------|---------|
| Proxy | 4000 | Route your agents here |
| REST API | 4001 | Dashboard reads data from here |
| Dashboard | 3000 | Web UI |
| TimescaleDB | 5432 | Event storage (PostgreSQL 16) |

**Environment variables** (read by the proxy; defaults live in `config/govrix.default.toml`):

| Name | Meaning |
|------|---------|
| `GOVRIX_DATABASE_URL` | Postgres/TimescaleDB connection string |
| `GOVRIX_PROXY_PORT`, `GOVRIX_API_PORT` | Proxy (4000) and API (4001) ports |
| `GOVRIX_PROXY_FAIL_OPEN` | Keep forwarding traffic when internal errors occur (default on) |
| `GOVRIX_PROXY__UPSTREAM_OPENAI`, `GOVRIX_PROXY__UPSTREAM_ANTHROPIC` | Upstream base URLs |
| `GOVRIX_API_KEY` | If set, the management API requires `Authorization: Bearer <key>` |
| `GOVRIX_LOG_LEVEL`, `GOVRIX_CONFIG`, `GOVRIX_POLICY_FILE` | Log level, config file path, policy YAML path |

**Upstream LLM key:** Govrix does not hold one. Your agent keeps using its own `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`, and the proxy forwards it unchanged.

Point an agent at it:

```bash
export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1
export ANTHROPIC_BASE_URL=http://localhost:4000/proxy/anthropic/v1
```

<details>
<summary><b>Connect your agents: per-framework examples and troubleshooting</b></summary>

#### Step 1 — Make sure Govrix Scout is running

```bash
# If you haven't started it yet:
docker compose -f docker/docker-compose.yml up -d

# Verify all services are up:
curl http://localhost:4001/health   # -> {"status":"ok"}
curl http://localhost:4001/ready    # -> {"status":"ready"}
```

---

#### Step 2 — Change ONE line in your agent code

Pick your framework below. Each example shows the **exact line** you need to change.

<details open>
<summary><b>Python — OpenAI SDK</b></summary>

**Before (direct to OpenAI):**
```python
from openai import OpenAI

client = OpenAI(api_key="sk-...")
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello"}]
)
```

**After (through Govrix Scout):**
```python
from openai import OpenAI

client = OpenAI(
    api_key="sk-...",
    base_url="http://localhost:4000/proxy/openai/v1",                   # ADD THIS LINE
    default_headers={"x-govrix-scout-agent-id": "my-agent-name"}       # OPTIONAL: name your agent
)
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello"}]
)
# Everything else stays exactly the same!
```

</details>

<details>
<summary><b>Python — Anthropic SDK</b></summary>

**After (through Govrix Scout):**
```python
from anthropic import Anthropic

client = Anthropic(
    api_key="sk-ant-...",
    base_url="http://localhost:4000/proxy/anthropic/v1",                 # ADD THIS LINE
    default_headers={"x-govrix-scout-agent-id": "my-claude-agent"}      # OPTIONAL
)
message = client.messages.create(
    model="claude-sonnet-4-20250514",
    max_tokens=1024,
    messages=[{"role": "user", "content": "Hello"}]
)
```

</details>

<details>
<summary><b>Python — LangChain</b></summary>

```python
from langchain_openai import ChatOpenAI

llm = ChatOpenAI(
    model="gpt-4",
    api_key="sk-...",
    base_url="http://localhost:4000/proxy/openai/v1",              # ADD THIS LINE
    default_headers={"x-govrix-scout-agent-id": "langchain-agent"} # OPTIONAL
)
response = llm.invoke("Hello")
```

</details>

<details>
<summary><b>Node.js — OpenAI SDK</b></summary>

```javascript
import OpenAI from "openai";

const client = new OpenAI({
  apiKey: "sk-...",
  baseURL: "http://localhost:4000/proxy/openai/v1",                   // ADD THIS LINE
  defaultHeaders: { "x-govrix-scout-agent-id": "my-node-agent" },    // OPTIONAL
});
const completion = await client.chat.completions.create({
  model: "gpt-4",
  messages: [{ role: "user", content: "Hello" }],
});
```

</details>

<details>
<summary><b>Python — CrewAI</b></summary>

```python
from crewai import Agent, LLM

llm = LLM(
    model="gpt-4",
    api_key="sk-...",
    base_url="http://localhost:4000/proxy/openai/v1"  # ADD THIS LINE
)
agent = Agent(role="Researcher", llm=llm, ...)
```

</details>

<details>
<summary><b>Environment Variable Method (works with ANY agent)</b></summary>

If your framework reads `OPENAI_BASE_URL` or `ANTHROPIC_BASE_URL` from the environment, you don't need to change any code at all:

```bash
export OPENAI_API_KEY=sk-...
export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1    # ADD THIS LINE

# For Anthropic agents:
export ANTHROPIC_API_KEY=sk-ant-...
export ANTHROPIC_BASE_URL=http://localhost:4000/proxy/anthropic/v1  # ADD THIS LINE
```

> **Tip:** Add these to your `.env` file so they persist across sessions.

</details>

<details>
<summary><b>cURL (for quick testing)</b></summary>

```bash
curl http://localhost:4000/proxy/openai/v1/chat/completions \
  -H "Authorization: Bearer sk-..." \
  -H "Content-Type: application/json" \
  -H "x-govrix-scout-agent-id: test-agent" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"Hello"}]}'
```

</details>

---

#### Step 3 — Open the Dashboard & Verify

1. Open **http://localhost:3000** in your browser
2. Run your agent (make at least one API call)
3. You should see the request appear in the dashboard within seconds

**Quick health check:**

```bash
curl http://localhost:4001/health   # -> {"status":"ok"}
curl http://localhost:4001/ready    # -> {"status":"ready"}
```

---

#### Troubleshooting

| Problem | Solution |
|---------|----------|
| `Connection refused` on port 4000 | Make sure Docker containers are running: `docker ps` |
| Agent not appearing in dashboard | Wait 5 seconds, then refresh. Check `x-govrix-scout-agent-id` header |
| API key errors | Your real API key must be valid — Govrix forwards it unchanged |
| Slow responses | First request may take ~1s (cold start). Subsequent requests add <5ms overhead |
| Dashboard blank on `:3000` | Ensure the dashboard container is running: `docker compose logs dashboard` |


</details>

<details>
<summary><b>Run the demo (Vercel Function + dashboard) locally</b></summary>

```bash
cd dashboard && npm ci && npm run build && cd ..
npx vercel dev          # serves dashboard/dist and api/index.ts
```

</details>

<details>
<summary><b>Project structure, database schema, REST API</b></summary>

### Project structure

```
govrix-scout/
├── crates/
│   ├── govrix-scout-proxy/        # Core: HTTP proxy (hyper) + REST API (axum) + policy engine
│   ├── govrix-scout-store/        # Database layer — PostgreSQL/TimescaleDB persistence (sqlx)
│   ├── govrix-scout-common/       # Shared types, config loading, error handling
│   ├── govrix-scout-cli/          # CLI binary (clap) — status, agents, events commands
│   └── govrix-scout-reports/      # Compliance report generation (HTML, JSON, PDF)
├── dashboard/                     # React 18 + TypeScript + Vite + Tailwind CSS + Recharts
│   └── src/
│       ├── pages/                 # 19 pages: overview, agents, events, costs, budgets, etc.
│       ├── api/                   # Typed API client, hooks (TanStack Query v5), types
│       └── components/            # Shared UI components
├── init/                          # idempotent SQL migrations 000-014 (events, agents, costs, budgets, sessions, traces, alerts)
├── docker/                        # Dockerfiles + docker-compose (dev, local, production)
├── k8s/                           # Kubernetes manifests (namespace, deployment, service, configmap, secrets)
├── config/                        # Default config (govrix.default.toml) + example policies (YAML)
├── scripts/                       # Setup, verification, DB reset, demo data seeding
├── docs/                          # Architecture plans and assets
└── tests/                         # Integration test directory
```

#### Database schema (TimescaleDB)

| Table | Purpose |
|-------|---------|
| `events` | Core audit log — every API call with tokens, cost, PII findings, lineage hash (hypertable) |
| `agents` | Agent registry — auto-discovered, tracks IP, models, error counts, last seen |
| `costs` | Cost aggregations by agent, model, and time period |
| `budget_config` | Budget rules — daily/monthly limits per agent or project |
| `budget_daily` | Budget enforcement state — tracks spend against limits |
| `projects` | Project isolation — group agents and events by project |
| `sessions` | Session tracking — group related requests in a conversation |
| `traces` | Distributed tracing data for debugging multi-step agent workflows |

#### REST API endpoints (port 4001)

| Endpoint | Description |
|----------|-------------|
| `GET /health`, `GET /ready` | Health and readiness checks |
| `GET /api/v1/events` | Query captured events with filters |
| `GET /api/v1/agents` | List discovered agents |
| `GET /api/v1/costs/summary`, `/breakdown`, `/timeseries` | Cost breakdowns by agent/model/time |
| `GET /api/v1/reports/types`, `POST /api/v1/reports/generate` | Generate compliance reports |
| `GET /api/v1/config` | Current proxy configuration |

Also: `/api/v1/traces`, `/alerts` (plus SSE stream), `/budgets/overview`, `/sessions`, `/compliance/{framework}`, `/risk/overview`, `/pii/activity`, `/policies`, `/kill-switch/*`. If `GOVRIX_API_KEY` is set, all `/api/v1/*` endpoints require Bearer auth.

### Build from source

> For contributors and developers. Most users should use [Quick Start](#quick-start) above.

#### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| **Rust + Cargo** | 1.75+ | [rustup.rs](https://rustup.rs) |
| **Node.js** | 20 LTS+ | [nodejs.org](https://nodejs.org) |
| **pnpm** | 9+ | `corepack enable && corepack prepare pnpm@latest --activate` |
| **Docker** | 25+ | [docker.com](https://www.docker.com/get-started/) |
| **C/C++ Linker** | — | See below |

<details>
<summary><b>C/C++ Linker installation (required for Rust)</b></summary>

| Platform | Command |
|----------|---------|
| **Windows** | Install [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with "Desktop development with C++" workload |
| **macOS** | `xcode-select --install` |
| **Ubuntu / Debian** | `sudo apt-get install build-essential` |
| **Fedora / RHEL** | `sudo dnf install gcc` |

</details>

#### Build and run

```bash
# 1. Clone
git clone https://github.com/manaspros/govrix-scout.git
cd govrix-scout

# 2. Start the database
docker compose -f docker/docker-compose.yml up -d postgres

# 3. Build the Rust proxy
cargo build --release --workspace

# 4. Run the proxy
export GOVRIX_DATABASE_URL=postgres://govrix:govrix_scout_dev@localhost:5432/govrix
RUST_LOG=info ./target/release/govrix-scout

# 5. Run the dashboard (separate terminal)
cd dashboard && pnpm install && pnpm dev
```

#### Hot reload

```bash
cargo install cargo-watch          # one-time setup
make docker-up                     # start database
make dev-proxy                     # proxy with hot-reload (terminal 1)
make dev-dashboard                 # dashboard with hot-reload (terminal 2)
```

#### Tests

```bash
cargo test --workspace             # Rust unit tests
cd dashboard && pnpm test          # Dashboard tests
make ci                            # Full CI pipeline
```

</details>

---

## 8. Status and roadmap

**Status:** v0.2.0 (see [CHANGELOG](CHANGELOG.md)). Proxy, API, 19-page dashboard, migrations 000 to 014, and Docker and Kubernetes manifests are in place. Report persistence and PDF export are stubbed in OSS.

**Next, roughly in order:**
- A benchmark harness to replace the "under 5 ms p99" claim with a measured number
- A command to verify a session's lineage-hash chain, and persisting chain state across restarts
- Policy enforcement beyond alert-only in OSS; PII masking before upstream (today in the Enterprise tier)
- Optional Redis-stream sink (tier 2) and ClickHouse (tier 3) for higher event rates

---


## Govrix Platform (Enterprise)

Scout gives you **visibility**. When you need **control**:

| Feature | Description |
|---------|-------------|
| **Real-time PII masking** | Redact sensitive data before it reaches upstream APIs |
| **Policy enforcement** | Block budget overruns, require human approval for sensitive operations |
| **Compliance reports** | One-click SOC 2, EU AI Act, HIPAA, FINRA report generation |
| **A2A identity** | Agent certificates and permission scoping via mTLS |
| **SSO + RBAC** | Okta, Azure AD, Google Workspace integration |
| **Unlimited scale** | No caps, multi-cluster, Kubernetes Helm charts |

<p align="center">
  <a href="https://govrix.dev"><b>govrix.dev</b></a> · <a href="mailto:hello@govrix.dev">Contact sales</a>
</p>

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide. Quick version:

```bash
./scripts/setup.sh    # install deps
make docker-up         # start database
make dev               # start everything with hot-reload
```

---

## License

[Apache 2.0](LICENSE) — free to use, modify, and distribute.

---

<p align="center">
  <a href="https://github.com/manaspros/govrix-scout">Star on GitHub</a> · <a href="https://github.com/manaspros/govrix-scout/issues">Report Bug</a> · <a href="https://github.com/manaspros/govrix-scout/issues">Request Feature</a> · <a href="CONTRIBUTING.md">Contribute</a>
</p>

<p align="center">
  Built by <a href="https://github.com/manaspros"><b>Manas</b></a>
</p>
