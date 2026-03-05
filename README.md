<p align="center">
  <img src="logo.png" alt="Govrix Scout Logo" width="200" />
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Govrix_Scout-AI%20Agent%20Governance-8B5CF6?style=for-the-badge&logo=data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0id2hpdGUiPjxwYXRoIGQ9Ik0xMiAyQzYuNDggMiAyIDYuNDggMiAxMnM0LjQ4IDEwIDEwIDEwIDEwLTQuNDggMTAtMTBTMTcuNTIgMiAxMiAyem0tMiAxNWwtNS01IDEuNDEtMS40MUwxMCAxNC4xN2w3LjU5LTcuNTlMMTkgOGwtOSA5eiIvPjwvc3ZnPg==&logoColor=white" alt="Govrix Scout" />
</p>

<h1 align="center">🛡️ Govrix Scout</h1>

<p align="center">
  <b>Open-Source AI Agent Observability, Governance & Compliance Platform</b>
</p>

<p align="center">
  <i>Know what your AI agents are doing — before your auditor asks.</i>
</p>

<br/>

<p align="center">
  <b>⭐️ If you find Govrix Scout useful, <a href="https://github.com/Govrix-AI/govrix-scout">please consider giving us a star!</a> It helps the open-source project grow. ⭐️</b>
</p>

<p align="center">
  Built with ❤️ by <a href="https://github.com/manaspros"><b>Manas</b></a>
</p>


<p align="center">
  <a href="#-quick-start"><img src="https://img.shields.io/badge/Setup-2%20Minutes-00C853?style=flat-square" alt="Setup Time" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache%202.0-blue?style=flat-square" alt="License" /></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.75+-DEA584?style=flat-square&logo=rust&logoColor=white" alt="Rust" /></a>
  <a href="https://react.dev"><img src="https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black" alt="React" /></a>
  <a href="https://www.timescale.com"><img src="https://img.shields.io/badge/TimescaleDB-PG16-FDB515?style=flat-square&logo=postgresql&logoColor=white" alt="TimescaleDB" /></a>
</p>

<p align="center">
  <a href="https://govrix.dev"><b>Website</b></a> ·
  <a href="https://docs.govrix.dev"><b>Docs</b></a> ·
  <a href="#-quick-start"><b>Getting Started</b></a> ·
  <a href="https://github.com/Govrix-AI/govrix-scout/issues"><b>Issues</b></a> ·
  <a href="#-govrix-platform-enterprise"><b>Enterprise</b></a>
</p>

<br/>

<p align="center">
  <img src="imgonline-com-ua-GIF-animation-2aVXJx6VtOK.gif" alt="Govrix Scout Dashboard Demo" width="800" />
</p>

---

## 🎯 What is Govrix Scout?

Govrix Scout is a **transparent reverse proxy** that sits between your AI agents and their APIs (OpenAI, Anthropic, etc.). It captures every request and response — **without touching your agent code**.

**One env var. Zero code changes. Full visibility.**

```bash
export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1
# That's it. Your agents keep working exactly as before.
```

Now every AI call is automatically **logged**, **costed**, **scanned for PII**, and **attributed** to the agent that made it.

```text
  ┌─────────────────┐       ┌────────────────────┐       ┌───────────────────┐
  │                 │       │   🛡️ Govrix Scout  │       │                   │
  │   Your Agents   │ ────► │     (:4000)        │ ────► │  OpenAI / Claude  │
  │                 │       └────────┬───────────┘       │                   │
  └─────────────────┘                │                   └───────────────────┘
                                     ▼
                            ┌────────────────┐
                            │  PostgreSQL    │
                            │  & Dashboard   │
                            └────────────────┘
```

## 🌟 Why Govrix?

- **Zero Friction** — No SDKs to install. Just change one environment variable (`OPENAI_BASE_URL`).
- **Complete Autonomy** — Agents are auto-discovered. No manual registration required.
- **Audit-Ready** — Cryptographically hashes every event in a Merkle chain for tamper evidence.
- **Privacy First** — Scans and flags 5 types of PII locally. Your data never leaves your infrastructure.

<br/>

---

## ✨ Core Features

| Feature                          | What it does                                                                          |
| -------------------------------- | ------------------------------------------------------------------------------------- |
| 🔍 **Agent Auto-Discovery**       | Every agent is automatically detected and catalogued — no manual registration, no SDK |
| 📋 **Full Event Logging**         | Every request/response captured with model, tokens, cost, latency, and tool calls     |
| 💰 **Cost Attribution**           | Track AI spend by agent, model, and time period — no more surprise bills              |
| 🔐 **PII Detection**              | Flags emails, phone numbers, SSNs, credit cards, and IPs in agent traffic             |
| 🔗 **Tamper-Evident Audit Trail** | SHA-256 Merkle hash chain proves event ordering and integrity                         |
| 📊 **Real-Time Dashboard**        | Overview, agents, events, costs, budgets, projects, and compliance reports            |
| 📡 **Streaming Support**          | Full SSE and chunked transfer support with minimal latency overhead                   |
| ⚙️ **Policy Engine**              | YAML-based rules to allow, block, or alert on agent behaviors                         |

<details>
<summary><b>View deep-dive capabilities</b></summary>
<br/>

- **Agent Identity & Tracking**: Tracks source IP, active status, error counts, and last models used.
- **Tamper Evidence Fields**: `session_id`, `timestamp`, `lineage_hash`, and `compliance_tag` are mandatory on every request.
- **Streaming Pipeline**: Tees SSE streams with under 5ms (p99) latency overhead using a Rust `hyper` hot-path.
- **Async Write Buffer**: Fire-and-forget channel batches writes to TimescaleDB every 100ms so I/O never blocks agent traffic.
- **Reporting**: Generates artifacts ready for SOC 2, HIPAA, EU AI Act, and FINRA auditors.

</details>

**Supported protocols:** OpenAI · Anthropic · MCP · A2A · Custom HTTP

<br/>

---

## 🚀 Quick Start

> **Requires:** [Docker](https://docker.com) & Docker Compose v2

### Option A: One-Line Install

```bash
# Linux / macOS
curl -sSL https://govrix.dev/install.sh | bash

# Windows (PowerShell as Admin)
iwr -useb https://raw.githubusercontent.com/Govrix-AI/govrix-scout/main/install.ps1 | iex
```

### Option B: Clone & Run

```bash
git clone https://github.com/Govrix-AI/govrix-scout.git
cd govrix-scout
docker compose -f docker/docker-compose.yml up -d
```

### What starts

| Service         | Port   | Purpose                        |
| --------------- | ------ | ------------------------------ |
| **Proxy**       | `4000` | Route your agents here         |
| **REST API**    | `4001` | Dashboard reads data from here |
| **Dashboard**   | `3000` | Web UI — open in browser       |
| **TimescaleDB** | `5432` | Event storage (PostgreSQL 16)  |

### 🔌 Add Govrix to Your Existing Agent Setup (Step-by-Step)

Govrix Scout works as a **transparent reverse proxy**. You change **one line** in your existing code — the API base URL — and every request flows through Govrix for logging, cost tracking, and compliance. **Your agent code, prompts, and logic stay exactly the same.**

> **How it works:** Instead of your agent calling `api.openai.com` directly, it calls `localhost:4000` (Govrix). Govrix logs the request and forwards it to the real API. The response comes back through Govrix and is returned to your agent unchanged.

---

#### Step 1 — Make sure Govrix Scout is running

```bash
# If you haven't started it yet:
docker compose -f docker/docker-compose.yml up -d

# Verify all services are up:
curl http://localhost:4001/health   # → {"status":"ok"}
curl http://localhost:4001/ready    # → {"status":"ready"}
```

You should see all 4 services running (Proxy on `:4000`, API on `:4001`, Dashboard on `:3000`, TimescaleDB on `:5432`).

---

#### Step 2 — Change ONE line in your agent code

Pick your framework below. Each example shows the **exact line** you need to change.

<details open>
<summary><b>🐍 Python — OpenAI SDK</b></summary>

**❌ BEFORE (direct to OpenAI):**
```python
from openai import OpenAI

client = OpenAI(api_key="sk-...")
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello"}]
)
```

**✅ AFTER (through Govrix Scout):**
```python
from openai import OpenAI

client = OpenAI(
    api_key="sk-...",                                                   # ← keep your real key
    base_url="http://localhost:4000/proxy/openai/v1",                   # ← ADD THIS LINE
    default_headers={"x-govrix-scout-agent-id": "my-agent-name"}       # ← OPTIONAL: name your agent
)
response = client.chat.completions.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Hello"}]
)
# Everything else stays exactly the same!
```

**What changed?** Only 2 lines added to the `OpenAI()` constructor: `base_url` and optionally `default_headers`. The rest of your code is untouched.

</details>

<details>
<summary><b>🐍 Python — Anthropic SDK</b></summary>

**❌ BEFORE (direct to Anthropic):**
```python
from anthropic import Anthropic

client = Anthropic(api_key="sk-ant-...")
message = client.messages.create(
    model="claude-sonnet-4-20250514",
    max_tokens=1024,
    messages=[{"role": "user", "content": "Hello"}]
)
```

**✅ AFTER (through Govrix Scout):**
```python
from anthropic import Anthropic

client = Anthropic(
    api_key="sk-ant-...",                                                # ← keep your real key
    base_url="http://localhost:4000/proxy/anthropic/v1",                 # ← ADD THIS LINE
    default_headers={"x-govrix-scout-agent-id": "my-claude-agent"}      # ← OPTIONAL
)
message = client.messages.create(
    model="claude-sonnet-4-20250514",
    max_tokens=1024,
    messages=[{"role": "user", "content": "Hello"}]
)
```

**What changed?** Added `base_url` pointing to Govrix's Anthropic proxy endpoint.

</details>

<details>
<summary><b>🐍 Python — LangChain</b></summary>

**❌ BEFORE:**
```python
from langchain_openai import ChatOpenAI

llm = ChatOpenAI(model="gpt-4", api_key="sk-...")
response = llm.invoke("Hello")
```

**✅ AFTER (through Govrix Scout):**
```python
from langchain_openai import ChatOpenAI

llm = ChatOpenAI(
    model="gpt-4",
    api_key="sk-...",
    base_url="http://localhost:4000/proxy/openai/v1",              # ← ADD THIS LINE
    default_headers={"x-govrix-scout-agent-id": "langchain-agent"} # ← OPTIONAL
)
response = llm.invoke("Hello")
```

**What changed?** Added `base_url` and optionally `default_headers` to the `ChatOpenAI()` constructor.

</details>

<details>
<summary><b>📦 Node.js — OpenAI SDK</b></summary>

**❌ BEFORE:**
```javascript
import OpenAI from "openai";

const client = new OpenAI({ apiKey: "sk-..." });
const completion = await client.chat.completions.create({
  model: "gpt-4",
  messages: [{ role: "user", content: "Hello" }],
});
```

**✅ AFTER (through Govrix Scout):**
```javascript
import OpenAI from "openai";

const client = new OpenAI({
  apiKey: "sk-...",
  baseURL: "http://localhost:4000/proxy/openai/v1",                   // ← ADD THIS LINE
  defaultHeaders: { "x-govrix-scout-agent-id": "my-node-agent" },    // ← OPTIONAL
});
const completion = await client.chat.completions.create({
  model: "gpt-4",
  messages: [{ role: "user", content: "Hello" }],
});
```

**What changed?** Added `baseURL` and optionally `defaultHeaders` to the constructor.

</details>

<details>
<summary><b>📦 Node.js — LangChain</b></summary>

**❌ BEFORE:**
```javascript
import { ChatOpenAI } from "@langchain/openai";
const model = new ChatOpenAI({ openAIApiKey: "sk-..." });
```

**✅ AFTER (through Govrix Scout):**
```javascript
import { ChatOpenAI } from "@langchain/openai";
const model = new ChatOpenAI({
  openAIApiKey: "sk-...",
  configuration: {
    baseURL: "http://localhost:4000/proxy/openai/v1",                 // ← ADD THIS LINE
    defaultHeaders: { "x-govrix-scout-agent-id": "langchain-agent" } // ← OPTIONAL
  }
});
```

**What changed?** Added `configuration.baseURL` and optionally `defaultHeaders`.

</details>

<details>
<summary><b>🤖 Python — CrewAI</b></summary>

**❌ BEFORE:**
```python
from crewai import Agent, LLM

llm = LLM(model="gpt-4", api_key="sk-...")
agent = Agent(role="Researcher", llm=llm, ...)
```

**✅ AFTER (through Govrix Scout):**
```python
from crewai import Agent, LLM

llm = LLM(
    model="gpt-4",
    api_key="sk-...",
    base_url="http://localhost:4000/proxy/openai/v1"  # ← ADD THIS LINE
)
agent = Agent(role="Researcher", llm=llm, ...)
```

**What changed?** Added `base_url` to the `LLM()` constructor.

</details>

<details>
<summary><b>🌐 Environment Variable Method (works with ANY agent)</b></summary>

If your framework reads `OPENAI_BASE_URL` or `ANTHROPIC_BASE_URL` from the environment, you don't need to change any code at all:

**❌ BEFORE (in your terminal / .env file):**
```bash
export OPENAI_API_KEY=sk-...
# No base URL set → calls go directly to api.openai.com
```

**✅ AFTER (in your terminal / .env file):**
```bash
export OPENAI_API_KEY=sk-...
export OPENAI_BASE_URL=http://localhost:4000/proxy/openai/v1    # ← ADD THIS LINE

# For Anthropic agents:
export ANTHROPIC_API_KEY=sk-ant-...
export ANTHROPIC_BASE_URL=http://localhost:4000/proxy/anthropic/v1  # ← ADD THIS LINE
```

**What changed?** Added one environment variable. Zero code changes required.

> **Tip:** Add these to your `.env` file so they persist across sessions.

</details>

<details>
<summary><b>🧪 cURL (for quick testing)</b></summary>

**❌ BEFORE (direct to OpenAI):**
```bash
curl https://api.openai.com/v1/chat/completions \
  -H "Authorization: Bearer sk-..." \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"Hello"}]}'
```

**✅ AFTER (through Govrix Scout):**
```bash
curl http://localhost:4000/proxy/openai/v1/chat/completions \
  -H "Authorization: Bearer sk-..." \
  -H "Content-Type: application/json" \
  -H "x-govrix-scout-agent-id: test-agent" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"Hello"}]}'
```

**What changed?** Replaced `https://api.openai.com` with `http://localhost:4000/proxy/openai`. Added optional agent ID header.

</details>

---

#### Step 3 — Open the Dashboard & Verify

1. Open **http://localhost:3000** in your browser
2. Run your agent (make at least one API call)
3. You should see the request appear in the dashboard within seconds

**Checklist — you know it's working when:**
- [ ] Dashboard shows your agent in the **Agents** tab
- [ ] Each API call appears in the **Events** tab with model, tokens, and cost
- [ ] The **Overview** page shows live metrics

**Quick health check commands:**

```bash
# Check all services are running
curl http://localhost:4001/health   # → {"status":"ok"}
curl http://localhost:4001/ready    # → {"status":"ready"}

# Test the proxy is forwarding correctly (replace with your key)
curl http://localhost:4000/proxy/openai/v1/chat/completions \
  -H "Authorization: Bearer sk-..." \
  -H "Content-Type: application/json" \
  -d '{"model":"gpt-4","messages":[{"role":"user","content":"ping"}]}'
```

---

#### ❓ Troubleshooting

| Problem                           | Solution                                                                       |
| --------------------------------- | ------------------------------------------------------------------------------ |
| `Connection refused` on port 4000 | Make sure Docker containers are running: `docker ps`                           |
| Agent not appearing in dashboard  | Wait 5 seconds, then refresh. Check `x-govrix-scout-agent-id` header           |
| API key errors                    | Your real API key must be valid — Govrix forwards it unchanged                 |
| Slow responses                    | First request may take ~1s (cold start). Subsequent requests add <5ms overhead |
| Dashboard blank on `:3000`        | Ensure the dashboard container is running: `docker compose logs dashboard`     |

<br/>

---

## 🛠️ Build from Source

> For contributors and developers. Most users should use [Quick Start](#-quick-start) above.

### Prerequisites

| Tool               | Version | Install                                                                                            |
| ------------------ | ------- | -------------------------------------------------------------------------------------------------- |
| **Rust + Cargo**   | 1.75+   | [rustup.rs](https://rustup.rs) — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **C/C++ Linker**   | —       | Required by Rust (see below)                                                                       |
| **Node.js**        | 20 LTS+ | [nodejs.org](https://nodejs.org)                                                                   |
| **pnpm**           | 9+      | `corepack enable && corepack prepare pnpm@latest --activate`                                       |
| **Docker**         | 25+     | [docker.com](https://www.docker.com/get-started/)                                                  |
| **Docker Compose** | v2+     | Included with Docker Desktop                                                                       |

#### C/C++ Linker (Required for Rust)

Rust needs a system linker to compile. Install one for your platform:

| Platform            | What to install                                                                                                                        | Command                                |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| **Windows**         | [Visual Studio Build Tools 2022](https://visualstudio.microsoft.com/visual-cpp-build-tools/) → "Desktop development with C++" workload | (GUI installer)                        |
| **macOS**           | Xcode Command Line Tools                                                                                                               | `xcode-select --install`               |
| **Ubuntu / Debian** | build-essential                                                                                                                        | `sudo apt-get install build-essential` |
| **Fedora / RHEL**   | gcc                                                                                                                                    | `sudo dnf install gcc`                 |

> ⚠️ **Windows users**: Install the **Build Tools** (not the full Visual Studio IDE). Select the **C++ workload** — the Rust linker (`link.exe`) is bundled inside it.

#### Verify Your Tools

Run this to confirm your environment is ready:

```bash
rustc --version && cargo --version && node --version && pnpm --version && docker --version
```

### Build & Run

```bash
# 1. Clone
git clone https://github.com/Govrix-AI/govrix-scout.git
cd govrix-scout

# 2. Start the database
docker compose -f docker/docker-compose.yml up -d postgres

# 3. Build the Rust proxy
cargo build --release --workspace

# 4. Run the proxy
export DATABASE_URL=postgres://govrix:govrix_scout_dev@localhost:5432/govrix
RUST_LOG=info ./target/release/govrix-scout

# 5. Run the dashboard (separate terminal)
cd dashboard
pnpm install
pnpm dev
```

### Development with hot-reload

```bash
cargo install cargo-watch          # one-time setup
make docker-up                     # start database
make dev-proxy                     # proxy with hot-reload (terminal 1)
make dev-dashboard                 # dashboard with hot-reload (terminal 2)
```

### Run tests

```bash
cargo test --workspace             # Rust unit tests
cd dashboard && pnpm test          # Dashboard tests
make ci                            # Full CI pipeline
```

<br/>

---

## 🗂️ Project Structure

```
govrix-scout/
├── crates/
│   ├── govrix-scout-common/       # Shared types, config, parsing
│   ├── govrix-scout-store/        # Database layer (sqlx)
│   ├── govrix-scout-proxy/        # Proxy (hyper) + API (axum) + policy engine
│   ├── govrix-scout-cli/          # CLI (clap)
│   └── govrix-scout-reports/      # Compliance report templates
├── dashboard/                     # React 18 + Vite + Recharts
├── docker/                        # Dockerfiles + compose
├── init/                          # SQL migrations
├── config/                        # Default config + policy examples
└── scripts/                       # Setup, seed data, verification
```

<br/>

---

## 🏗️ Technical Architecture (Bird's-Eye View)

Here's how Govrix Scout processes your agent traffic with near-zero latency overhead:

```mermaid
flowchart TB
    subgraph ClientLayer ["Client Layer"]
        direction LR
        Agent["🤖 AI Agents\n(LangChain, CrewAI, etc.)"]
        Browser["💻 Web Dashboard\n(React + Vite)"]
    end

    subgraph GovrixScout ["🛡️ Govrix Scout (Rust)"]
        direction TB
        Proxy["⚡ Proxy Server\n(hyper, port 4000)\nHot-path routing"]
        API["⚙️ Management API\n(axum, port 4001)\nREST & Config"]
        PolicyEngine["📋 Policy Engine\nYAML Rules & Budgets"]
        EventChannel["📥 Event Channel\n(Bounded mpsc)"]
        Writer["📝 Background Writer\nAsync Batching"]

        Proxy <-->|"Hook"| PolicyEngine
        Proxy -->|"Fire & Forget"| EventChannel
        EventChannel --> Writer
    end

    subgraph DataLayer ["Data Layer"]
        DB[("🐘 PostgreSQL 16\n(TimescaleDB, port 5432)")]
    end

    subgraph UpstreamLayer ["Upstream Providers"]
        direction LR
        OpenAI["OpenAI API"]
        Anthropic["Anthropic API"]
    end

    %% Connections
    Agent <==>|"1. Request/Response"| Proxy
    Proxy <==>|"2. Forwarded Traffic"| OpenAI
    Proxy <==>|"2. Forwarded Traffic"| Anthropic

    API <==>|"Reads/Writes Config"| DB
    Writer -->|"3. Async Batched Writes"| DB

    Browser <==>|"REST API Calls"| API

    classDef proxy fill:#8B5CF6,stroke:#fff,stroke-width:2px,color:#fff
    classDef db fill:#FDB515,stroke:#fff,stroke-width:2px,color:#000
    classDef upstream fill:#10a37f,stroke:#fff,stroke-width:2px,color:#fff

    class Proxy,API proxy
    class DB db
    class OpenAI,Anthropic upstream
```

- **Fail-Open Design:** The proxy uses a bounded non-blocking `mpsc` channel. If the database goes down, events are dropped but your agent traffic **continues to flow**.
- **Minimal Latency:** The hot-path is built directly on `hyper` (bypassing generic web routing overhead) for sub-5ms processing latency.
- **Asynchronous Storage:** Database writes and aggregations are batched in the background by a dedicated Tokio task.

<br/>

---

## 🏢 Govrix Platform (Enterprise)

Scout gives you **visibility**. When you need **control**:

- **Real-time PII masking** — redact data before it reaches APIs
- **Policy enforcement** — block budget overruns, require human approval
- **Compliance reports** — one-click SOC 2, EU AI Act, HIPAA, FINRA
- **A2A identity** — agent certificates and permission scoping
- **SSO + RBAC** — Okta, Azure AD, Google Workspace
- **Unlimited scale** — no caps, multi-cluster, Kubernetes Helm charts

<p align="center">
  <a href="https://govrix.dev"><b>govrix.dev</b></a> · <a href="mailto:hello@govrix.dev">Contact sales</a>
</p>

<br/>

---

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide. Quick version:

```bash
./scripts/setup.sh    # install deps
make docker-up         # start database
make dev               # start everything with hot-reload
```

<br/>

---

## 📜 License

[Apache 2.0](LICENSE) — free to use, modify, and distribute. Forever.

---

<p align="center">
  <a href="https://github.com/Govrix-AI/govrix-scout">⭐ Star on GitHub</a> · <a href="https://github.com/Govrix-AI/govrix-scout/issues">🐛 Report Bug</a> · <a href="https://github.com/Govrix-AI/govrix-scout/issues">💡 Request Feature</a> · <a href="CONTRIBUTING.md">🤝 Contribute</a>
</p>

---

> The initial README and documentation were drafted during an early collaboration that did not continue. Govrix Scout is now solely built and maintained by [Manas](https://github.com/manaspros).
