# Integration Test Environment — Simulation Plan

## Context

Both repos (Scout 188 tests, Platform 91 tests) have strong unit test coverage but zero end-to-end integration tests. We need a simulated environment that exercises all 15 product features without real LLM API keys. This plan documents the architecture, components, and 15 test scenarios.

**Deliverable:** `docs/plans/2026-02-19-integration-test-environment.md` in the Scout repo (shared infrastructure).

---

## Architecture

```
  ┌──────────────┐     ┌─────────────────────────┐     ┌───────────────┐
  │ test-runner   │────▶│  scout / govrix proxy    │────▶│ mock-upstream  │
  │ (scenarios +  │     │  (system under test)     │     │ (fake LLM)    │
  │  assertions)  │     │  :4000 proxy :4001 API   │     │ :9999         │
  └──────────────┘     └─────────────────────────┘     └───────────────┘
         │                        │
         │              ┌─────────┘
         ▼              ▼
     ┌──────────────────────┐
     │   postgres            │
     │   (TimescaleDB :5432) │
     └──────────────────────┘
```

All containers orchestrated by `docker-compose-test-{scout,platform}.yml`. Test runner generates traffic AND validates assertions.

---

## Components

### 1. Mock Upstream LLM Server (Rust/axum, port 9999)

Location: `tests/integration/mock-upstream/`

Routes:
- `POST /v1/chat/completions` — OpenAI format (streaming + non-streaming)
- `POST /v1/messages` — Anthropic format (streaming + non-streaming)
- `POST /admin/configure` — set latency, error rate, token counts
- `GET /admin/stats` — request counters by agent
- `GET /health` — liveness

Returns synthetic responses with configurable token counts and costs. Supports tool-call responses when request contains `tools`.

### 2. Test Runner + Agent Simulator (Rust binary)

Location: `tests/integration/test-runner/`

8 agent personas: analyst-001 (OpenAI/high-cost), coder-002 (OpenAI/cheap), reviewer-003 (Anthropic), intern-004 (sends PII), runaway-005 (burst loops), shadow-006 (no agent header), enterprise-007 (mTLS), multi-tenant-008 (tenant header).

Runs 15 scenarios sequentially. Each: setup → action → wait for flush → assertions via API queries.

### 3. Docker Compose (two variants)

- `docker-compose-test-scout.yml` — postgres + mock-upstream + scout-proxy + test-runner
- `docker-compose-test-platform.yml` — postgres + mock-upstream + govrix-server + test-runner

Key override: `AGENTMESH_PROXY__UPSTREAM_OPENAI=http://mock-upstream:9999` redirects proxy to mock.

---

## Critical Prerequisite

**Parameterize upstream URLs.** Current `resolve_upstream_base()` in `agentmesh-proxy/src/proxy/upstream.rs` returns hardcoded `https://api.openai.com`. Must read from config instead. The config fields `upstream_openai` / `upstream_anthropic` already exist in `agentmesh.default.toml`.

---

## 15 Test Scenarios

| # | Feature | Mode | Setup | Action | Key Assertion |
|---|---------|------|-------|--------|--------------|
| 1 | Audit Trail | Both | Clean DB | 4 requests from 2 agents | Events have lineage_hash chain, session_id, compliance_tag |
| 2 | Policy Enforcement | Platform | Load block-anthropic rule | 1 OpenAI + 1 Anthropic request | OpenAI=200, Anthropic=403 with "block:" tag |
| 3 | PII Masking | Platform | pii_masking_enabled=true | Request with SSN+email in body | Event tagged "warn:pii-detected" |
| 4 | Cost Attribution | Both | Mock returns 100+50 tokens | Requests from 2 agents, 2 models | cost_usd populated, gpt-4o > gpt-4o-mini |
| 5 | Budget Caps | Platform | runaway-005 limit=500 tokens | 6 requests (push over limit) | First 4 pass, later blocked with "budget-exceeded" |
| 6 | mTLS Identity | Platform | Enterprise license | POST /certs/issue | Returns valid cert_pem + key_pem |
| 7 | Shadow Discovery | Both | No registered agents | Requests with only Bearer token | Agent auto-discovered from API key pattern |
| 8 | Multi-Tenant | Platform | Enterprise | Create 2 tenants via API | GET /tenants returns 3 (default + 2 new) |
| 9 | Compliance Tags | Both | Varies by mode | 3 requests | Every event has non-empty compliance_tag |
| 10 | Hot-Reload | Platform | Empty rules | Load rule → block → clear rule → allow | Policy changes without restart |
| 11 | Session Tracking | Both | Clean DB | 5 from agent-A, 2 from agent-B | Same session_id within agent, different across |
| 12 | Vendor Distribution | Both | Clean DB | 3 OpenAI + 2 Anthropic | Events have correct provider field |
| 13 | Incident Response | Both | 10+ events seeded | Query with filters + pagination | Filtered results correct, event detail complete |
| 14 | License Tiers | Platform | 3 runs: Community/Starter/Enterprise | Check /license + /certs/issue | Community=no features, Enterprise=all features |
| 15 | Fail-Open | Both | Broken DB (wrong password) | 3 proxy requests | All return 200 (proxy forwards despite DB failure) |

---

## File Structure

```
tests/integration/
├── mock-upstream/
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/
│       ├── main.rs
│       ├── openai.rs
│       ├── anthropic.rs
│       ├── admin.rs
│       └── sse.rs
├── test-runner/
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/
│       ├── main.rs
│       ├── agents.rs
│       ├── assertions.rs
│       └── scenarios/
│           ├── mod.rs
│           └── s01..s15.rs
├── fixtures/
│   ├── policies/
│   └── licenses/
├── docker-compose-test-scout.yml
├── docker-compose-test-platform.yml
└── Makefile
```

## Implementation Order

1. **Parameterize upstream URLs** in Scout proxy (blocking prerequisite)
2. **Build mock-upstream** server (OpenAI + Anthropic endpoints)
3. **Build test runner framework** (health polling, scenario orchestrator, assertions)
4. **Implement scenarios 1, 4, 11, 12, 13** (Scout-only features first)
5. **Wire docker-compose-test-scout.yml** + `make test-scout`
6. **Implement scenarios 2, 3, 5, 6, 8, 9, 10, 14** (Platform features)
7. **Wire docker-compose-test-platform.yml** + `make test-platform`
8. **Implement scenarios 7, 15** (special setup: shadow discovery, fail-open)
9. **CI integration** — GitHub Actions workflow for both repos

## Timing Budget: <5 min total

| Phase | Time |
|-------|------|
| Docker build (cached) | 30s |
| Postgres + migrations | 15s |
| Services startup | 7s |
| 15 scenarios | ~180s |
| Teardown | 5s |
| **Total** | **~4 min** |

## Verification

- `make test-scout` — runs Scout scenarios, exits 0
- `make test-platform` — runs all 15 scenarios, exits 0
- Both deterministic (no real API keys, fixed mock responses)
- CI runs on every PR
