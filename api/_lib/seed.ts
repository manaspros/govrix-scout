/**
 * Deterministic sample-traffic generator for the Vercel demo.
 *
 * Mirrors the shapes of the real Rust models/migrations:
 *   events   -> init/001_create_events.sql + 009_agent_tracing.sql
 *   agents   -> init/002_create_agents.sql
 *   sessions -> init/010_sessions.sql, traces -> init/011_traces.sql
 *   alerts   -> init/012_anomaly_alerts.sql
 * The lineage hash is the real algorithm from
 * crates/govrix-scout-proxy/src/events.rs::compute_lineage_hash:
 *   sha256("{prev}|{event_id}|{agent_id}|{timestamp_ms}"), first prev = "GENESIS".
 *
 * Everything is generated in memory from a fixed PRNG seed, anchored to the
 * current hour so the data always looks fresh. Nothing here talks to a database.
 */
import { createHash } from 'crypto'

// ── PRNG ─────────────────────────────────────────────────────────────────────
function mulberry32(seed: number) {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let t = a
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

export function computeLineageHash(prev: string, eventId: string, agentId: string, tsMs: number): string {
  return createHash('sha256').update(`${prev}|${eventId}|${agentId}|${tsMs}`).digest('hex')
}

// ── Static catalogue ─────────────────────────────────────────────────────────
// USD per 1M tokens (input, output). Sample prices, rounded.
const PRICES: Record<string, [number, number]> = {
  'gpt-4o': [2.5, 10],
  'gpt-4o-mini': [0.15, 0.6],
  'gpt-4.1': [2, 8],
  'claude-sonnet-4-20250514': [3, 15],
  'claude-3-5-haiku-20241022': [0.8, 4],
}
const PROVIDER_OF: Record<string, string> = {
  'gpt-4o': 'openai',
  'gpt-4o-mini': 'openai',
  'gpt-4.1': 'openai',
  'claude-sonnet-4-20250514': 'anthropic',
  'claude-3-5-haiku-20241022': 'anthropic',
}
const UPSTREAM: Record<string, string> = {
  openai: 'https://api.openai.com/v1/chat/completions',
  anthropic: 'https://api.anthropic.com/v1/messages',
}

interface AgentDef {
  id: string
  name: string
  description: string
  framework: string
  models: string[]
  weight: number
  tools: [string, string][] // [tool_name, mcp_server]
  prompts: string[]
  ip: string
  piiRate: number
}

const AGENT_DEFS: AgentDef[] = [
  {
    id: 'support-bot', name: 'Support Bot', description: 'Tier-1 customer support triage', framework: 'direct_api',
    models: ['gpt-4o-mini', 'gpt-4o'], weight: 24, ip: '10.0.4.21', piiRate: 0.16,
    tools: [['search_docs', 'kb-mcp'], ['create_ticket', 'zendesk-mcp']],
    prompts: ['Customer reports login loop after password reset. Summarise and suggest next step.', 'Draft a reply about the delayed refund for order #48213.', 'Classify this ticket: "app crashes when exporting PDF".'],
  },
  {
    id: 'research-crew', name: 'Research Crew', description: 'Multi-agent market research (CrewAI)', framework: 'crewai',
    models: ['claude-sonnet-4-20250514'], weight: 14, ip: '10.0.7.8', piiRate: 0.03,
    tools: [['web_search', 'browser-mcp'], ['read_file', 'fs-mcp'], ['summarize_pdf', 'fs-mcp']],
    prompts: ['Compare pricing pages of three observability vendors.', 'Summarise the EU AI Act Article 12 record-keeping duties.', 'Find recent benchmarks for Rust HTTP proxies.'],
  },
  {
    id: 'code-reviewer', name: 'Code Reviewer', description: 'Pull-request review assistant (LangChain)', framework: 'langchain',
    models: ['gpt-4.1', 'gpt-4o'], weight: 16, ip: '10.0.2.44', piiRate: 0.02,
    tools: [['github_pr_diff', 'github-mcp'], ['post_review_comment', 'github-mcp']],
    prompts: ['Review this diff for SQL injection risks.', 'Suggest tests for the new retry logic in client.rs.', 'Summarise the PR in three bullets.'],
  },
  {
    id: 'billing-assistant', name: 'Billing Assistant', description: 'Invoice lookups and dunning drafts', framework: 'direct_api',
    models: ['gpt-4o-mini'], weight: 12, ip: '10.0.5.17', piiRate: 0.3,
    tools: [['lookup_invoice', 'billing-mcp'], ['send_email', 'smtp-mcp']],
    prompts: ['Draft a polite reminder for invoice INV-2291, 14 days overdue.', 'Explain the proration on this customer\'s upgrade.', 'List invoices over $5k that are unpaid.'],
  },
  {
    id: 'data-pipeline-agent', name: 'Data Pipeline Agent', description: 'Schema drift triage for the warehouse (MCP client)', framework: 'mcp_client',
    models: ['claude-3-5-haiku-20241022', 'claude-sonnet-4-20250514'], weight: 13, ip: '10.0.9.3', piiRate: 0.05,
    tools: [['run_sql', 'warehouse-mcp'], ['list_tables', 'warehouse-mcp'], ['read_file', 'fs-mcp']],
    prompts: ['Column `amount` changed from int to decimal in orders. Impact?', 'Generate a backfill plan for table events_2026_q1.', 'Why did the nightly dbt run fail?'],
  },
  {
    id: 'hr-onboarding-bot', name: 'HR Onboarding Bot', description: 'New-hire Q&A and checklist generation', framework: 'autogen',
    models: ['claude-3-5-haiku-20241022'], weight: 8, ip: '10.0.3.60', piiRate: 0.34,
    tools: [['search_docs', 'kb-mcp'], ['create_task', 'asana-mcp']],
    prompts: ['Build a first-week checklist for a backend engineer in Berlin.', 'What is the parental leave policy for contractors?', 'Summarise equipment ordering steps.'],
  },
  {
    id: 'sales-outreach', name: 'Sales Outreach', description: 'Personalised outbound email drafts', framework: 'custom',
    models: ['gpt-4o'], weight: 9, ip: '10.0.6.12', piiRate: 0.22,
    tools: [['crm_lookup', 'crm-mcp'], ['send_email', 'smtp-mcp']],
    prompts: ['Write a 3-line intro email for the VP Engineering at a fintech.', 'Rewrite this follow-up to be shorter.', 'Which accounts opened our last email twice?'],
  },
  {
    id: 'legacy-etl-agent', name: 'Legacy ETL Agent', description: 'Old nightly ETL helper. Blocked by kill switch.', framework: 'custom',
    models: ['gpt-4o'], weight: 4, ip: '10.0.1.99', piiRate: 0.12,
    tools: [['run_sql', 'warehouse-mcp']],
    prompts: ['Generate a MERGE statement for the customers dimension.', 'Retry failed batch 8841 with smaller chunks.'],
  },
]

const PII_TYPES = ['email', 'phone', 'ssn', 'credit_card', 'ip_address']
const PII_WEIGHTS = [0.42, 0.22, 0.08, 0.1, 0.18]
const DETECTORS = ['cost_zscore', 'latency_p99', 'exfil', 'security', 'fanout', 'error_rate', 'behavioral']

// ── Public types ─────────────────────────────────────────────────────────────
export type Json = Record<string, any>

export interface Dataset {
  anchor: number
  startedAt: number
  events: Json[] // newest first
  agents: Json[]
  sessions: Json[]
  traces: Json[]
  alerts: Json[]
  budgets: Json[]
  killed: Json[]
}

// ── Generator ────────────────────────────────────────────────────────────────
export function generate(now = Date.now()): Dataset {
  const HOUR = 3600_000
  const anchor = Math.floor(now / HOUR) * HOUR
  const rnd = mulberry32(20260301)
  const pick = <T,>(arr: T[]): T => arr[Math.floor(rnd() * arr.length)]
  const between = (a: number, b: number) => a + rnd() * (b - a)
  const int = (a: number, b: number) => Math.floor(between(a, b + 1))
  const uuid = () => {
    const h = Array.from({ length: 32 }, () => Math.floor(rnd() * 16).toString(16))
    h[12] = '4'
    h[16] = '89ab'[Math.floor(rnd() * 4)]
    const s = h.join('')
    return `${s.slice(0, 8)}-${s.slice(8, 12)}-${s.slice(12, 16)}-${s.slice(16, 20)}-${s.slice(20)}`
  }
  const weighted = <T,>(items: T[], w: (t: T) => number): T => {
    const total = items.reduce((s, i) => s + w(i), 0)
    let r = rnd() * total
    for (const i of items) {
      r -= w(i)
      if (r <= 0) return i
    }
    return items[items.length - 1]
  }
  const iso = (ms: number) => new Date(ms).toISOString()
  const round6 = (n: number) => Math.round(n * 1e6) / 1e6

  const events: Json[] = []
  const sessions: Json[] = []
  const traces: Json[] = []
  const SESSIONS = 160
  const SPAN_DAYS = 7

  for (let s = 0; s < SESSIONS; s++) {
    const def = weighted(AGENT_DEFS, a => a.weight)
    const blockedAgent = def.id === 'legacy-etl-agent'
    // Bias toward recent: u^1.6 spreads fewer sessions into the distant past.
    const ageMs = Math.pow(rnd(), 1.6) * SPAN_DAYS * 24 * HOUR
    // Blocked agent stopped ~2 days ago.
    const start = blockedAgent ? anchor - between(2.2, 6) * 24 * HOUR : anchor - ageMs - 60_000
    const sessionId = uuid()
    const traceId = uuid()
    const model = pick(def.models)
    const provider = PROVIDER_OF[model]
    const spans: Json[] = []
    let t = start

    const make = (kind: string, extra: Json, parent: string | null) => {
      const id = uuid()
      const spanId = uuid()
      const e: Json = {
        id,
        session_id: sessionId,
        agent_id: def.id,
        timestamp: iso(t),
        latency_ms: null,
        direction: 'outbound',
        method: 'POST',
        upstream_target: UPSTREAM[provider],
        provider,
        model: null,
        status_code: 200,
        finish_reason: null,
        payload: {},
        raw_size_bytes: int(900, 9000),
        input_tokens: null,
        output_tokens: null,
        total_tokens: null,
        cost_usd: null,
        pii_detected: [],
        tools_called: [],
        lineage_hash: '',
        compliance_tag: 'pass:all',
        tags: { env: 'demo', team: def.id.split('-')[0] },
        error_message: null,
        event_kind: kind,
        span_id: spanId,
        parent_span_id: parent,
        trace_id: traceId,
        tool_name: null,
        tool_args: null,
        tool_result: null,
        mcp_server: null,
        risk_score: Math.round(between(3, 24) * 10) / 10,
        external_trace_id: null,
        created_at: iso(t),
        ...extra,
      }
      spans.push(e)
      return e
    }

    const root = make('session.start', { upstream_target: 'govrix://session', provider: 'custom', status_code: null, payload: { agent: def.id } }, null)
    t += int(40, 400)
    const calls = int(2, 6)
    for (let c = 0; c < calls; c++) {
      const inTok = int(180, 3200)
      const outTok = int(40, 1400)
      const [pin, pout] = PRICES[model]
      const cost = round6((inTok * pin + outTok * pout) / 1e6)
      const latency = Math.round(between(450, 3800) * (model.includes('mini') || model.includes('haiku') ? 0.55 : 1))
      const usesTool = def.tools.length > 0 && rnd() < 0.45
      const prompt = pick(def.prompts)
      let tag = 'pass:all'
      let status = 200
      let kind = 'llm.response'
      let err: string | null = null
      let pii: Json[] = []
      let risk = Math.round(between(4, 28) * 10) / 10
      if (rnd() < def.piiRate) {
        const n = rnd() < 0.7 ? 1 : 2
        pii = Array.from({ length: n }, () => ({
          pii_type: weighted(PII_TYPES, x => PII_WEIGHTS[PII_TYPES.indexOf(x)]),
          location: rnd() < 0.8 ? 'request.messages[0].content' : 'response.choices[0].message.content',
          confidence: Math.round(between(0.82, 0.99) * 100) / 100,
        }))
        risk = Math.round(between(48, 72) * 10) / 10
        tag = 'warn:pii-detected'
        if (rnd() < 0.25 || (blockedAgent && c === calls - 1)) {
          tag = 'block:pii-policy'
          kind = 'policy.block'
          status = 403
          risk = Math.round(between(80, 96) * 10) / 10
          err = 'Blocked by policy pii-detection: sensitive data in outbound prompt'
        }
      } else if (rnd() < 0.04) {
        status = pick([429, 500, 503])
        err = status === 429 ? 'upstream rate limit exceeded' : 'upstream error'
        risk = Math.round(between(18, 40) * 10) / 10
      }
      const e = make(kind, {
        model,
        latency_ms: latency,
        status_code: status,
        finish_reason: err ? null : usesTool ? 'tool_calls' : 'stop',
        input_tokens: err ? 0 : inTok,
        output_tokens: err ? 0 : outTok,
        total_tokens: err ? 0 : inTok + outTok,
        cost_usd: err ? 0 : cost,
        pii_detected: pii,
        compliance_tag: tag,
        error_message: err,
        risk_score: risk,
        payload: { request: { model, messages: [{ role: 'user', content: prompt }] }, demo: true },
        tools_called: usesTool ? [def.tools[0][0]] : [],
      }, root.span_id)
      t += latency
      if (usesTool && !err) {
        const [tool, mcp] = pick(def.tools)
        t += int(20, 120)
        const inv = make('tool.invoke', { tool_name: tool, mcp_server: mcp, provider: 'mcp', upstream_target: `mcp://${mcp}`, tool_args: { query: prompt.slice(0, 40) }, risk_score: Math.round(between(5, 35) * 10) / 10 }, e.span_id)
        t += int(60, 900)
        make('tool.result', { tool_name: tool, mcp_server: mcp, provider: 'mcp', upstream_target: `mcp://${mcp}`, latency_ms: int(60, 900), tool_result: { ok: true, rows: int(1, 40) } }, inv.span_id)
      }
      t += int(300, 4000)
      if (err && kind === 'policy.block') break
    }
    if (rnd() < 0.6 && !spans.some(e => e.event_kind === 'policy.block')) {
      make('session.end', { upstream_target: 'govrix://session', provider: 'custom', status_code: null }, root.span_id)
    }

    // Chain lineage hashes in time order (this is the tamper-evident part).
    spans.sort((a, b) => Date.parse(a.timestamp) - Date.parse(b.timestamp))
    let prev = 'GENESIS'
    for (const e of spans) {
      prev = computeLineageHash(prev, e.id, e.agent_id, Date.parse(e.timestamp))
      e.lineage_hash = prev
    }

    const cost = spans.reduce((a, e) => a + (e.cost_usd ?? 0), 0)
    const first = Date.parse(spans[0].timestamp)
    const last = Date.parse(spans[spans.length - 1].timestamp)
    const blocked = spans.some(e => e.event_kind === 'policy.block')
    const ongoing = anchor - last < 20 * 60_000 && !blocked
    const peak = Math.max(...spans.map(e => e.risk_score ?? 0))
    const status = blocked ? 'stopped' : ongoing ? 'running' : spans.some(e => (e.status_code ?? 200) >= 500) ? 'failed' : 'completed'
    const killed = blockedAgent && blocked

    sessions.push({
      session_id: sessionId,
      agent_id: def.id,
      trace_id: traceId,
      status: killed ? 'killed' : ongoing ? 'active' : 'completed',
      started_at: iso(first),
      last_event_at: iso(last),
      created_at: iso(first),
      event_count: spans.length,
      total_cost_usd: round6(cost),
      killed_at: killed ? iso(last) : null,
      killed_by: killed ? 'policy-engine' : null,
      kill_reason: killed ? 'PII policy violation (auto-kill)' : null,
      integrity_hash: 'sha256:' + spans[spans.length - 1].lineage_hash,
    })
    traces.push({
      trace_id: traceId,
      root_agent_id: def.id,
      root_agent: def.id,
      task_description: spans.find(e => e.payload?.request)?.payload.request.messages[0].content ?? null,
      status,
      stopped_by: blocked ? 'policy' : null,
      error_message: status === 'failed' ? 'upstream error' : null,
      started_at: iso(first),
      completed_at: ongoing ? null : iso(last),
      ended_at: ongoing ? undefined : iso(last),
      duration_ms: last - first,
      created_at: iso(first),
      total_cost_usd: round6(cost),
      peak_risk_score: peak,
      event_count: spans.length,
      span_count: spans.length,
      agent_count: 1,
      external_trace_id: null,
      metadata: { demo: true },
      _spans: spans,
    })
    events.push(...spans)
  }

  events.sort((a, b) => Date.parse(b.timestamp) - Date.parse(a.timestamp))
  for (const e of events) {
    // Dashboard-facing aliases (the SPA reads `kind` / `protocol`).
    e.kind = e.event_kind
    e.protocol = e.provider
  }
  // Trace spans in the shape the dashboard's TraceSpan expects.
  for (const tr of traces) {
    tr.spans = tr._spans.map((e: Json) => ({
      id: e.span_id,
      span_id: e.span_id,
      trace_id: e.trace_id,
      parent_span_id: e.parent_span_id ?? undefined,
      agent_id: e.agent_id,
      event_kind: e.event_kind,
      tool_name: e.tool_name ?? undefined,
      mcp_server: e.mcp_server ?? undefined,
      model: e.model ?? undefined,
      started_at: e.timestamp,
      ended_at: e.latency_ms ? iso(Date.parse(e.timestamp) + e.latency_ms) : undefined,
      latency_ms: e.latency_ms ?? undefined,
      input_tokens: e.input_tokens ?? undefined,
      output_tokens: e.output_tokens ?? undefined,
      total_tokens: e.total_tokens ?? undefined,
      cost_usd: e.cost_usd ?? undefined,
      risk_score: e.risk_score ?? undefined,
      tool_args: e.tool_args ?? undefined,
      tool_result: e.tool_result ?? undefined,
      error_message: e.error_message ?? undefined,
      status_code: e.status_code ?? undefined,
    }))
    delete tr._spans
  }
  sessions.sort((a, b) => Date.parse(b.last_event_at) - Date.parse(a.last_event_at))
  traces.sort((a, b) => Date.parse(b.started_at) - Date.parse(a.started_at))

  // ── Agents (aggregated from events, like the real upsert path) ─────────────
  const agents: Json[] = AGENT_DEFS.map(def => {
    const mine = events.filter(e => e.agent_id === def.id)
    const llm = mine.filter(e => e.model)
    const last = mine[0]
    const first = mine[mine.length - 1]
    const errors = mine.filter(e => (e.status_code ?? 200) >= 400)
    let status = 'active'
    if (def.id === 'legacy-etl-agent') status = 'blocked'
    else if (last && anchor - Date.parse(last.timestamp) > 6 * HOUR) status = 'idle'
    else if (def.id === 'billing-assistant') status = 'error'
    const tokensIn = llm.reduce((a, e) => a + (e.input_tokens ?? 0), 0)
    const tokensOut = llm.reduce((a, e) => a + (e.output_tokens ?? 0), 0)
    return {
      id: def.id,
      name: def.name,
      description: def.description,
      framework: def.framework,
      agent_type: def.framework,
      status,
      labels: { env: 'demo', owner: 'platform-team' },
      total_requests: llm.length,
      total_tokens_in: tokensIn,
      total_tokens_out: tokensOut,
      total_tokens: tokensIn + tokensOut,
      total_cost_usd: round6(llm.reduce((a, e) => a + (e.cost_usd ?? 0), 0)),
      last_model_used: llm[0]?.model ?? null,
      last_seen_at: last?.timestamp ?? null,
      last_error_at: errors[0]?.timestamp ?? null,
      error_count: errors.length,
      source_ip: def.ip,
      first_seen_at: first?.timestamp ?? null,
      created_at: first?.timestamp ?? null,
      updated_at: last?.timestamp ?? null,
    }
  })

  // ── Alerts (shape of init/012_anomaly_alerts.sql) ──────────────────────────
  const alerts: Json[] = []
  const alertSpecs: [string, string, string, Json][] = [
    ['cost_zscore', 'warn', 'billing-assistant', { message: 'Hourly spend 3.4 sigma above 7-day baseline', z_score: 3.4 }],
    ['latency_p99', 'warn', 'research-crew', { message: 'p99 latency 2.1x baseline', p99_ms: 7420 }],
    ['exfil', 'critical', 'hr-onboarding-bot', { message: 'Large outbound payload containing 3 SSN-like patterns', bytes: 48211 }],
    ['security', 'critical', 'legacy-etl-agent', { message: 'Prompt contains credit card number; request blocked', rule: 'pii-detection' }],
    ['fanout', 'info', 'research-crew', { message: 'Agent spawned 6 sub-agents in 60s', spawned: 6 }],
    ['error_rate', 'warn', 'data-pipeline-agent', { message: 'Upstream 5xx rate 9% over last 15 min', rate: 0.09 }],
    ['behavioral', 'info', 'sales-outreach', { message: 'New tool crm_lookup used for the first time', tool: 'crm_lookup' }],
    ['cost_zscore', 'critical', 'support-bot', { message: 'Single session cost $1.82, 12x agent median', session_cost: 1.82 }],
    ['security', 'warn', 'code-reviewer', { message: 'Prompt injection pattern in tool result', pattern: 'ignore previous instructions' }],
    ['latency_p99', 'info', 'support-bot', { message: 'Latency recovered to baseline', p99_ms: 2100 }],
  ]
  for (let i = 0; i < 24; i++) {
    const [detector, severity, agentId, details] = alertSpecs[i % alertSpecs.length]
    const ts = anchor - Math.pow(rnd(), 1.4) * 3 * 24 * HOUR
    const ev = events.find(e => e.agent_id === agentId)
    alerts.push({
      id: uuid(),
      timestamp: iso(ts),
      agent_id: agentId,
      session_id: ev?.session_id ?? null,
      detector: DETECTORS.includes(detector) ? detector : 'legacy',
      severity,
      score: Math.round(between(severity === 'critical' ? 0.85 : severity === 'warn' ? 0.55 : 0.25, severity === 'critical' ? 0.99 : severity === 'warn' ? 0.84 : 0.5) * 100) / 100,
      details,
      acknowledged_at: i % 5 === 0 ? iso(ts + 15 * 60_000) : null,
      created_at: iso(ts),
    })
  }
  alerts.sort((a, b) => Date.parse(b.timestamp) - Date.parse(a.timestamp))

  // ── Budgets (rolling last 24h used as "today") ─────────────────────────────
  const since24 = anchor - 24 * HOUR
  const limits: Record<string, [number, number, number]> = {
    'support-bot': [400_000, 4, 80],
    'research-crew': [250_000, 5, 100],
    'code-reviewer': [300_000, 3, 60],
    'billing-assistant': [120_000, 0.5, 10],
    'data-pipeline-agent': [200_000, 2, 40],
    'hr-onboarding-bot': [100_000, 0.4, 8],
    'sales-outreach': [150_000, 1.5, 30],
    'legacy-etl-agent': [50_000, 1, 20],
  }
  const budgets: Json[] = [
    { agent_id: '__global__', name: 'All agents (global)', status: 'active', tokens_used_today: 0, cost_used_today: 0, daily_token_limit: 1_500_000, daily_cost_limit_usd: 100, monthly_cost_limit_usd: 2000 },
  ]
  for (const def of AGENT_DEFS) {
    const mine = events.filter(e => e.agent_id === def.id && Date.parse(e.timestamp) >= since24)
    const [tl, cl, ml] = limits[def.id]
    budgets.push({
      agent_id: def.id,
      name: def.name,
      status: agents.find(a => a.id === def.id)!.status,
      tokens_used_today: mine.reduce((a, e) => a + (e.total_tokens ?? 0), 0),
      cost_used_today: round6(mine.reduce((a, e) => a + (e.cost_usd ?? 0), 0)),
      daily_token_limit: tl,
      daily_cost_limit_usd: cl,
      monthly_cost_limit_usd: ml,
    })
  }
  const g = budgets[0]
  g.tokens_used_today = budgets.slice(1).reduce((a, b) => a + b.tokens_used_today, 0)
  g.cost_used_today = round6(budgets.slice(1).reduce((a, b) => a + b.cost_used_today, 0))

  const killed = sessions.filter(s => s.status === 'killed')
  return { anchor, startedAt: now, events, agents, sessions, traces, alerts, budgets, killed }
}

export { AGENT_DEFS, PRICES }
