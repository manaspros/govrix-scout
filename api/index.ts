/**
 * Govrix Scout — read-only demo API for Vercel Functions.
 *
 * One function serves every route the dashboard calls (vercel.json rewrites
 * /api/*, /health, /ready, /metrics here with the original path in `?path=`).
 * Response shapes follow the real Rust handlers in
 * crates/govrix-scout-proxy/src/api/handlers/*.rs, plus the extra aliases the
 * dashboard reads (e.g. `kind`/`protocol`, `overall_score`).
 *
 * Data is generated in memory by ./_lib/seed.ts. Writes (PUT/POST) succeed but
 * only live in the current function instance: this is a demo, not a database.
 */
import { generate, AGENT_DEFS, type Dataset, type Json } from './_lib/seed'

let DS: Dataset | null = null
const BOOT = Date.now()
function ds(): Dataset {
  // Regenerate hourly so timestamps stay fresh; in-memory edits reset with it.
  if (!DS || Date.now() - DS.startedAt > 3600_000) DS = generate()
  return DS
}

const CORS: Record<string, string> = {
  'access-control-allow-origin': '*',
  'access-control-allow-methods': 'GET,POST,PUT,DELETE,OPTIONS',
  'access-control-allow-headers': 'authorization,content-type',
}
function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'content-type': 'application/json', 'cache-control': 'no-store', ...CORS },
  })
}
const err = (status: number, error: string, extra: Json = {}) => json({ error, ...extra }, status)

const VERSION = '0.1.0-demo'

// ── helpers ──────────────────────────────────────────────────────────────────
const ms = (s: string | null | undefined): number | undefined => {
  if (!s) return undefined
  const t = Date.parse(s)
  return Number.isNaN(t) ? undefined : t
}
function num(q: URLSearchParams, k: string, d: number, min: number, max: number) {
  const v = Number(q.get(k))
  return Number.isFinite(v) && q.get(k) !== null && q.get(k) !== '' ? Math.min(max, Math.max(min, v)) : d
}
const sum = (a: Json[], f: (x: Json) => number) => a.reduce((s, x) => s + f(x), 0)
const r6 = (n: number) => Math.round(n * 1e6) / 1e6
const isLlm = (e: Json) => !!e.model && (e.event_kind === 'llm.response' || e.event_kind === 'policy.block')

function range(q: URLSearchParams, defDays = 7) {
  const to = ms(q.get('to') ?? q.get('until')) ?? Date.now()
  const days = q.get('days') ? num(q, 'days', defDays, 1, 90) : defDays
  const from = ms(q.get('from') ?? q.get('since')) ?? to - days * 86400_000
  return { from, to }
}

function filterEvents(q: URLSearchParams, base?: Json[]): Json[] {
  const d = ds()
  const agent = q.get('agent_id')
  const provider = q.get('provider') ?? q.get('protocol')
  const model = q.get('model')
  const kind = q.get('kind') ?? q.get('event_kind')
  const tag = q.get('compliance_tag')
  const from = ms(q.get('from') ?? q.get('since'))
  const to = ms(q.get('to') ?? q.get('until'))
  return (base ?? d.events).filter(e => {
    const t = Date.parse(e.timestamp)
    return (
      (!agent || e.agent_id === agent) &&
      (!provider || e.provider === provider) &&
      (!model || e.model === model) &&
      (!kind || e.event_kind === kind) &&
      (!tag || e.compliance_tag.startsWith(tag)) &&
      (from === undefined || t >= from) &&
      (to === undefined || t < to)
    )
  })
}

function paginate(rows: Json[], q: URLSearchParams) {
  const limit = num(q, 'limit', 50, 1, 500)
  const offset = q.get('offset') ? num(q, 'offset', 0, 0, 1e9) : q.get('page') ? (num(q, 'page', 1, 1, 1e6) - 1) * limit : 0
  return { data: rows.slice(offset, offset + limit), total: rows.length, limit, offset }
}

function percentile(vals: number[], p: number): number | null {
  if (!vals.length) return null
  const s = [...vals].sort((a, b) => a - b)
  return s[Math.min(s.length - 1, Math.floor(p * s.length))]
}

function agentView(a: Json) {
  return a
}

// ── compliance (ported from handlers/platform.rs::build_compliance_controls) ──
function control(id: string, name: string, description: string, passing: boolean, metric: number, evidence: string) {
  const score = passing && metric > 0 ? 100 : passing ? 75 : 0
  return { id, name, status: score >= 80 ? 'pass' : score >= 50 ? 'partial' : 'fail', score, description, evidence }
}
function complianceReport(framework: string) {
  const d = ds()
  const total = d.events.length
  const agents = d.agents.length
  const pii = d.events.filter(e => e.pii_detected.length > 0).length
  const blocks = d.events.filter(e => e.compliance_tag.startsWith('block:')).length
  const high = d.events.filter(e => (e.risk_score ?? 0) > 75).length
  const budgets = d.budgets.filter(b => b.agent_id !== '__global__').length
  const C: Record<string, any[]> = {
    soc2: [
      control('CC6.1', 'Logical Access Controls', 'Agent identification and authentication tracking', agents > 0, agents, `${agents} agents tracked with identity fingerprinting`),
      control('CC7.2', 'System Monitoring', 'Continuous monitoring of AI agent activity', total > 0, total, `${total} events captured in audit log`),
      control('CC6.7', 'Data Classification', 'PII detection and classification in agent traffic', true, pii, `${pii} PII instances detected and classified`),
      control('CC7.4', 'Incident Response', 'Policy enforcement and automated blocking', true, blocks, `${blocks} policy blocks enforced`),
      control('CC8.1', 'Change Management', 'Agent lifecycle tracking and risk scoring', high === 0 || total > 100, high, `${high} high-risk events flagged for review`),
      control('CC9.1', 'Risk Mitigation', 'Budget controls and cost governance', budgets > 0, budgets, `${budgets} budget policies configured`),
    ],
    'eu-ai-act': [
      control('ART-9', 'Risk Management System', 'Continuous risk scoring of AI agent operations', total > 0, total, `${total} operations risk-scored; ${high} flagged high-risk`),
      control('ART-12', 'Record Keeping', 'Complete audit trail of all AI agent interactions', total > 0, total, `${total} events in tamper-evident audit log with lineage hashing`),
      control('ART-13', 'Transparency', 'AI system identification and capability disclosure', agents > 0, agents, `${agents} agents registered with type, framework, and API targets`),
      control('ART-14', 'Human Oversight', 'Kill switch and policy-based intervention capability', true, blocks, `${blocks} automated interventions; kill switch available for all agents`),
      control('ART-10', 'Data Governance', 'PII detection and data flow monitoring', true, pii, `${pii} PII instances detected across agent traffic`),
    ],
    hipaa: [
      control('164.312(a)', 'Access Control', 'Unique agent identification and session tracking', agents > 0, agents, `${agents} agents with unique identifiers and session isolation`),
      control('164.312(b)', 'Audit Controls', 'Hardware, software, and procedural audit mechanisms', total > 0, total, `${total} audit events with SHA-256 lineage hashing`),
      control('164.312(c)', 'Integrity Controls', 'Protection against improper alteration or destruction', total > 0, total, `Merkle-chain integrity verification across ${total} events`),
      control('164.312(d)', 'Person Authentication', 'Agent identity verification', agents > 0, agents, `${agents} agents authenticated via fingerprinting`),
      control('164.308(a)(5)', 'Security Awareness', 'PII and PHI detection in AI agent traffic', true, pii, `${pii} PII/PHI instances detected and flagged`),
      control('164.308(a)(6)', 'Security Incident Procedures', 'Automated policy enforcement and incident response', true, blocks, `${blocks} security incidents automatically blocked`),
    ],
    'nist-800-53': [
      control('AC-2', 'Account Management', 'AI agent account lifecycle management', agents > 0, agents, `${agents} agent accounts managed with status tracking`),
      control('AU-2', 'Audit Events', 'Selection of auditable events from AI agent operations', total > 0, total, `${total} events captured across 15 event kinds`),
      control('AU-3', 'Content of Audit Records', 'Audit records contain sufficient detail for forensic analysis', total > 0, total, 'Events include timestamps, agent ID, session ID, lineage hash, and compliance tags'),
      control('RA-5', 'Vulnerability Monitoring', 'Continuous risk scoring and anomaly detection', true, high, `${high} high-risk events identified through continuous scoring`),
      control('SI-4', 'System Monitoring', 'Real-time monitoring of AI agent information flows', total > 0, total, `${total} events monitored in real-time with PII and risk detection`),
      control('IR-4', 'Incident Handling', 'Automated incident response via policy engine', true, blocks, `${blocks} incidents handled by the policy engine`),
    ],
  }
  const controls = C[framework]
  if (!controls) return null
  const score = Math.round((controls.reduce((s, c) => s + c.score, 0) / controls.length) * 10) / 10
  const status = score >= 80 ? 'compliant' : score >= 50 ? 'partial' : 'non-compliant'
  return { framework, status, score, overall_score: score / 100, generated_at: new Date().toISOString(), controls }
}

// ── session recording (flat shape the dashboard expects + Rust `data`/`events`) ──
function sessionRecording(id: string) {
  const d = ds()
  const s = d.sessions.find(x => x.session_id === id)
  if (!s) return null
  const evs = d.events.filter(e => e.session_id === id).sort((a, b) => Date.parse(a.timestamp) - Date.parse(b.timestamp))
  return {
    data: s,
    ...s,
    ended_at: s.status === 'active' ? undefined : s.last_event_at,
    event_count: evs.length,
    events: evs.map(e => ({
      id: e.id,
      agent_id: e.agent_id,
      timestamp: e.timestamp,
      kind: e.event_kind,
      event_kind: e.event_kind,
      provider: e.provider,
      model: e.model ?? undefined,
      input_tokens: e.input_tokens ?? undefined,
      output_tokens: e.output_tokens ?? undefined,
      latency_ms: e.latency_ms ?? undefined,
      status_code: e.status_code ?? undefined,
      error_message: e.error_message ?? undefined,
      risk_score: e.risk_score,
      compliance_tag: e.compliance_tag,
      cost_usd: e.cost_usd,
      tool_name: e.tool_name,
      lineage_hash: e.lineage_hash,
    })),
  }
}

// ── SSE (demo: replays synthetic traffic for ~25s, then the client reconnects) ──
function sse(kind: 'events' | 'alerts'): Response {
  const enc = new TextEncoder()
  let timer: ReturnType<typeof setInterval> | undefined
  let n = 0
  const stream = new ReadableStream({
    start(c) {
      c.enqueue(enc.encode(': govrix demo stream\n\n'))
      timer = setInterval(() => {
        const d = ds()
        n++
        if (n > 12) {
          clearInterval(timer)
          try { c.close() } catch { /* already closed */ }
          return
        }
        if (kind === 'events') {
          const src = d.events[Math.floor(Math.random() * Math.min(60, d.events.length))]
          const ev = { id: crypto.randomUUID(), timestamp: new Date().toISOString(), event_kind: src.event_kind, agent_id: src.agent_id, tool_name: src.tool_name ?? undefined, model: src.model ?? undefined, risk_score: src.risk_score, latency_ms: src.latency_ms ?? undefined, session_id: src.session_id }
          c.enqueue(enc.encode(`data: ${JSON.stringify(ev)}\n\n`))
        } else if (n % 4 === 0) {
          const a = d.alerts[Math.floor(Math.random() * d.alerts.length)]
          c.enqueue(enc.encode(`event: alert\ndata: ${JSON.stringify({ ...a, id: crypto.randomUUID(), timestamp: new Date().toISOString(), acknowledged_at: null })}\n\n`))
        } else c.enqueue(enc.encode(': ping\n\n'))
      }, 2000)
    },
    cancel() { if (timer) clearInterval(timer) },
  })
  return new Response(stream, { headers: { 'content-type': 'text/event-stream', 'cache-control': 'no-store, no-transform', connection: 'keep-alive', ...CORS } })
}

// ── router ───────────────────────────────────────────────────────────────────
async function handle(req: Request): Promise<Response> {
  const url = new URL(req.url)
  const q = url.searchParams
  let path = q.get('path') ?? url.pathname.replace(/^\/api\/index/, '')
  q.delete('path')
  path = '/' + path.replace(/^\/+|\/+$/g, '')
  const seg = path.split('/').filter(Boolean)
  const m = req.method
  if (m === 'OPTIONS') return new Response(null, { status: 204, headers: CORS })
  const d = ds()
  const body = async (): Promise<Json> => { try { return await req.json() } catch { return {} } }

  // Health
  if (path === '/health') return json({ status: 'ok', version: VERSION, db: 'demo (in-memory seed)', proxy: 'not running (demo)', uptime_secs: Math.round((Date.now() - BOOT) / 1000), mode: 'demo' })
  if (path === '/ready') return json({ status: 'ready', database: 'connected', db: 'demo seed', uptime_seconds: Math.round((Date.now() - BOOT) / 1000), version: VERSION, mode: 'demo' })
  if (path === '/metrics') {
    const llm = d.events.filter(isLlm)
    return new Response(
      `# demo metrics derived from seeded data\ngovrix_events_total ${d.events.length}\ngovrix_agents_active ${d.agents.filter(a => a.status === 'active').length}\ngovrix_cost_usd_total ${r6(sum(llm, e => e.cost_usd ?? 0))}\n`,
      { headers: { 'content-type': 'text/plain; version=0.0.4', ...CORS } },
    )
  }
  if (seg[0] !== 'api' || seg[1] !== 'v1') return err(404, 'not found', { path })
  const r = seg.slice(2) // after /api/v1
  const [a, b, c] = r

  // Events
  if (a === 'events') {
    if (b === 'stream') return sse('events')
    if (b === 'sessions' && c) {
      const rows = d.events.filter(e => e.session_id === c).sort((x, y) => Date.parse(x.timestamp) - Date.parse(y.timestamp))
      return json({ data: rows, events: rows, total: rows.length, session_id: c })
    }
    if (b) {
      const e = d.events.find(x => x.id === b)
      return e ? json({ data: e }) : err(404, 'event not found', { id: b })
    }
    return json(paginate(filterEvents(q), q))
  }

  // Agents
  if (a === 'agents') {
    if (!b) {
      let rows = d.agents.filter(x => x.status !== 'retired' || q.get('status') === 'retired')
      const st = q.get('status'); const s = q.get('search')?.toLowerCase()
      if (st) rows = d.agents.filter(x => x.status === st)
      if (s) rows = rows.filter(x => x.id.includes(s) || (x.name ?? '').toLowerCase().includes(s))
      return json({ ...paginate(rows.map(agentView), q), limit: num(q, 'limit', 100, 1, 500) })
    }
    const ag = d.agents.find(x => x.id === b)
    if (b === '__global__' && c === 'budget') { /* falls through to budget below */ }
    else if (!ag) return err(404, 'agent not found', { id: b })
    if (!c) {
      if (m === 'PUT') {
        const p = await body()
        Object.assign(ag!, Object.fromEntries(['name', 'description', 'labels'].filter(k => k in p).map(k => [k, p[k]])))
      }
      return json({ data: ag, ...ag })
    }
    if (c === 'retire' && m === 'POST') { ag!.status = 'retired'; return json({ data: { id: b, status: 'retired' }, status: 'retired' }) }
    if (c === 'events') return json(paginate(filterEvents(q, d.events.filter(e => e.agent_id === b)), q))
    if (c === 'runs') { const rows = d.traces.filter(t => t.root_agent_id === b); return json({ data: rows, total: rows.length }) }
    if (c === 'violations') { const rows = d.events.filter(e => e.agent_id === b && e.compliance_tag.startsWith('block:')); return json({ data: rows, total: rows.length }) }
    if (c === 'budget') {
      const bd = d.budgets.find(x => x.agent_id === b)
      if (m === 'PUT') {
        const p = await body()
        const row = bd ?? (d.budgets.push({ agent_id: b, name: ag?.name ?? b, status: 'active', tokens_used_today: 0, cost_used_today: 0 }), d.budgets[d.budgets.length - 1])
        for (const k of ['daily_token_limit', 'daily_cost_limit_usd', 'monthly_cost_limit_usd']) if (k in p) row[k] = p[k]
        return json({ data: { agent_id: b, daily_token_limit: row.daily_token_limit, daily_cost_limit_usd: row.daily_cost_limit_usd, monthly_cost_limit_usd: row.monthly_cost_limit_usd }, message: 'budget updated (demo: not persisted across instances)' })
      }
      if (m === 'DELETE') return json({ data: { agent_id: b, deleted: true } })
      if (!bd) return err(404, 'no budget configured', { id: b })
      return json({ data: { agent_id: b, config: { daily_token_limit: bd.daily_token_limit, daily_cost_limit_usd: bd.daily_cost_limit_usd, monthly_cost_limit_usd: bd.monthly_cost_limit_usd }, usage: { tokens_used_today: bd.tokens_used_today, cost_used_today: bd.cost_used_today } } })
    }
    if (c === 'project') return json({ data: { agent_id: b, project_id: 'demo' } })
  }

  // Costs
  if (a === 'costs') {
    const { from, to } = range(q)
    const rows = filterEvents(q, d.events.filter(isLlm)).filter(e => { const t = Date.parse(e.timestamp); return t >= from && t < to })
    if (b === 'summary') {
      const lat = rows.map(e => e.latency_ms).filter((x): x is number => typeof x === 'number')
      return json({ data: { from: new Date(from).toISOString(), to: new Date(to).toISOString(), total_requests: rows.length, total_input_tokens: sum(rows, e => e.input_tokens ?? 0), total_output_tokens: sum(rows, e => e.output_tokens ?? 0), total_cost_usd: r6(sum(rows, e => e.cost_usd ?? 0)), avg_latency_ms: lat.length ? sum(lat.map(x => ({ x })), o => o.x) / lat.length : null, p99_latency_ms: percentile(lat, 0.99) } })
    }
    if (b === 'breakdown') {
      const gb = q.get('group_by') ?? 'agent'
      const key = (e: Json) => (gb === 'model' ? e.model : gb === 'protocol' || gb === 'provider' ? e.provider : e.agent_id)
      const groups = new Map<string, Json[]>()
      for (const e of rows) { const k = String(key(e)); groups.set(k, [...(groups.get(k) ?? []), e]) }
      const data = [...groups].map(([group_key, g]) => ({ group_key, request_count: g.length, total_input_tokens: sum(g, e => e.input_tokens ?? 0), total_output_tokens: sum(g, e => e.output_tokens ?? 0), total_cost_usd: r6(sum(g, e => e.cost_usd ?? 0)), avg_latency_ms: sum(g, e => e.latency_ms ?? 0) / g.length })).sort((x, y) => y.total_cost_usd - x.total_cost_usd)
      return json({ data, total: data.length, from: new Date(from).toISOString(), to: new Date(to).toISOString(), group_by: gb })
    }
    if (b === 'timeseries') {
      const step = q.get('granularity') === 'hour' ? 3600_000 : 86400_000
      const buckets = new Map<number, Json[]>()
      for (const e of rows) { const k = Math.floor(Date.parse(e.timestamp) / step) * step; buckets.set(k, [...(buckets.get(k) ?? []), e]) }
      const data = [...buckets].sort((x, y) => x[0] - y[0]).map(([k, g]) => ({ bucket: new Date(k).toISOString(), timestamp: new Date(k).toISOString(), request_count: g.length, requests: g.length, total_input_tokens: sum(g, e => e.input_tokens ?? 0), total_output_tokens: sum(g, e => e.output_tokens ?? 0), tokens: sum(g, e => e.total_tokens ?? 0), total_cost_usd: r6(sum(g, e => e.cost_usd ?? 0)), cost_usd: r6(sum(g, e => e.cost_usd ?? 0)) }))
      return json({ data, total: data.length, from: new Date(from).toISOString(), to: new Date(to).toISOString() })
    }
  }

  // Budgets
  if (a === 'budgets' && b === 'overview') return json({ data: d.budgets, total: d.budgets.length })

  // Projects (single demo project)
  if (a === 'projects') {
    const proj = { id: 'demo', name: 'Demo project', description: 'Seeded sample project', created_at: new Date(d.anchor - 7 * 86400_000).toISOString(), agent_count: d.agents.length }
    if (!b) return json({ data: [proj], total: 1 })
    if (b !== 'demo') return err(404, 'project not found', { id: b })
    if (c === 'agents') return json({ data: d.agents, total: d.agents.length })
    if (c === 'costs') { const rows = d.events.filter(isLlm); return json({ data: { total_requests: rows.length, total_cost_usd: r6(sum(rows, e => e.cost_usd ?? 0)) } }) }
    return json({ data: proj })
  }

  // Alerts
  if (a === 'alerts') {
    if (b === 'stream') return sse('alerts')
    if (b && c === 'acknowledge' && m === 'POST') {
      const al = d.alerts.find(x => x.id === b)
      if (!al || al.acknowledged_at) return err(404, 'alert not found or already acknowledged', { id: b })
      al.acknowledged_at = new Date().toISOString()
      return json({ data: { id: b, acknowledged: true } })
    }
    if (b) { const al = d.alerts.find(x => x.id === b); return al ? json({ data: al }) : err(404, 'alert not found', { id: b }) }
    const sev = q.get('severity'); const det = q.get('detector'); const ag = q.get('agent_id'); const since = ms(q.get('since'))
    const limit = num(q, 'limit', 100, 1, 1000)
    const rows = d.alerts.filter(x => (!sev || x.severity === sev) && (!det || x.detector === det) && (!ag || x.agent_id === ag) && (since === undefined || Date.parse(x.timestamp) >= since)).slice(0, limit)
    return json({ data: rows, total: rows.length, limit })
  }

  // Traces
  if (a === 'traces') {
    if (b) {
      const t = d.traces.find(x => x.trace_id === b)
      return t ? json({ data: t }) : err(404, 'trace not found', { trace_id: b })
    }
    const ag = q.get('agent_id'); const st = q.get('status'); const since = ms(q.get('since')); const until = ms(q.get('until'))
    const rows = d.traces.filter(t => (!ag || t.root_agent_id === ag) && (!st || t.status === st) && (since === undefined || Date.parse(t.started_at) >= since) && (until === undefined || Date.parse(t.started_at) < until))
    const limit = num(q, 'limit', 25, 1, 200); const offset = num(q, 'offset', 0, 0, 1e9)
    // List view omits spans, like the real list handler.
    return json({ data: rows.slice(offset, offset + limit).map(({ spans: _s, ...t }) => t), total: rows.length, limit, offset })
  }

  // Reports (stubbed in the real OSS API too)
  if (a === 'reports') {
    if (b === 'types') return json({ data: [
      { id: 'usage_summary', name: 'Usage Summary', description: 'Aggregate request counts, token usage, and costs over a time period', formats: ['json', 'pdf'], format: 'json' },
      { id: 'cost_breakdown', name: 'Cost Breakdown', description: 'Detailed cost analysis by agent, model, or provider', formats: ['json', 'pdf'], format: 'json' },
      { id: 'agent_inventory', name: 'Agent Inventory', description: 'All discovered agents with framework, models and activity', formats: ['json', 'pdf'], format: 'json' },
    ], total: 3 })
    if (b === 'generate' && m === 'POST') {
      const p = await body()
      const id = crypto.randomUUID()
      return json({ id, report_type: p.report_type ?? 'usage_summary', status: 'pending', created_at: new Date().toISOString(), data: { report_id: id, status: 'queued', report_type: p.report_type, format: p.format ?? 'json', note: 'Demo: report generation is stubbed (same as the OSS API).' } }, 202)
    }
    if (!b) return json({ data: [], total: 0, note: 'Report persistence is not available in the OSS build or this demo.' })
  }

  // Config (shape of handlers/config.rs, values from config/govrix.default.toml)
  if (a === 'config') return json({ data: { proxy: { port: 4000, bind: '0.0.0.0', fail_open: true, upstream_timeout_ms: 30000, max_body_tee_bytes: 1048576, upstream_openai: 'https://api.openai.com', upstream_anthropic: 'https://api.anthropic.com' }, api: { port: 4001, bind: '0.0.0.0', cors_enabled: true, cors_origins: ['*'] }, database: { url: 'postgres://***@demo-seed/govrix', max_connections: 10, min_connections: 2, connect_timeout_secs: 5 }, retention: { events_days: 30, cost_days: 90 }, telemetry: { log_level: 'info' }, mode: 'demo' } })

  // Platform
  if (a === 'platform' && b === 'health') return json({ status: 'ok', tier: 'oss', license_tier: 'oss', max_agents: 25, version: VERSION, uptime_seconds: Math.round((Date.now() - BOOT) / 1000), database: 'connected', policy_enabled: true, pii_masking_enabled: false, compliance_enabled: true, a2a_identity_enabled: false, mtls_enabled: false, audit_trail_enabled: true, budget_tracking_enabled: true, features: { event_logging: true, agent_registry: true, session_tracking: true, budget_controls: true, pii_detection: true, risk_scoring: true, compliance_reports: true, kill_switch: true }, mode: 'demo' })
  if (a === 'platform' && b === 'license') return json({ tier: 'community', max_agents: 25, retention_days: 30, policy_enabled: true, pii_masking_enabled: false, compliance_enabled: true, a2a_identity_enabled: false })
  if (a === 'tenants') return err(404, 'tenants are not part of the OSS API')
  if (a === 'compliance' && b) {
    const rep = complianceReport(b)
    return rep ? json(rep) : err(400, 'unsupported framework', { supported: ['soc2', 'eu-ai-act', 'hipaa', 'nist-800-53'] })
  }

  if (a === 'risk' && b === 'overview') {
    const withRisk = d.events.filter(e => e.risk_score != null)
    const avg = withRisk.length ? sum(withRisk, e => e.risk_score) / withRisk.length : 0
    const days = new Map<string, Json[]>()
    for (const e of withRisk) { const k = e.timestamp.slice(0, 10); days.set(k, [...(days.get(k) ?? []), e]) }
    const trend = [...days].sort((x, y) => x[0].localeCompare(y[0])).slice(-7).map(([day, g]) => ({ day, score: Math.round((sum(g, e => e.risk_score) / g.length) * 10) / 10, event_count: g.length }))
    const flagged = d.events.filter(e => (e.risk_score ?? 0) > 50 || e.compliance_tag.startsWith('warn:') || e.compliance_tag.startsWith('block:')).slice(0, 20)
    const alerts = flagged.map(e => ({ id: e.id, agent_id: e.agent_id, agent: e.agent_id, timestamp: e.timestamp, risk_score: e.risk_score, compliance_tag: e.compliance_tag, event_kind: e.event_kind, severity: e.compliance_tag.startsWith('block:') ? 'critical' : (e.risk_score ?? 0) > 60 ? 'high' : 'medium', message: e.error_message ?? `${e.compliance_tag} on ${e.event_kind}` }))
    const warnings = d.events.filter(e => e.compliance_tag.startsWith('warn:')).length
    const blocks = d.events.filter(e => e.compliance_tag.startsWith('block:')).length
    const pii24 = d.events.filter(e => e.pii_detected.length && Date.parse(e.timestamp) > Date.now() - 86400_000).length
    const hi = d.events.filter(e => (e.risk_score ?? 0) > 75).length
    return json({ risk_score: Math.round(avg * 10) / 10, risk_label: avg >= 75 ? 'critical' : avg >= 50 ? 'high' : avg >= 25 ? 'medium' : 'low', alerts, trend, stats: { warnings, blocks, pii_detections_24h: pii24, high_risk_events: hi, total_alerts: warnings + blocks, critical: blocks, high: hi, medium: warnings, low: 0, policy_violations_24h: blocks } })
  }

  if (a === 'pii' && b === 'activity') {
    const hits = d.events.filter(e => e.pii_detected.length > 0)
    const types: Record<string, number> = {}
    for (const e of hits) for (const p of e.pii_detected) types[p.pii_type] = (types[p.pii_type] ?? 0) + 1
    const colors: Record<string, string> = { email: '#3b82f6', phone: '#22c55e', ssn: '#ef4444', credit_card: '#f97316', ip_address: '#a855f7' }
    const limit = num(q, 'limit', 50, 1, 500); const offset = num(q, 'offset', 0, 0, 1e9)
    const action = (e: Json) => (e.compliance_tag.startsWith('block:') ? 'blocked' : e.compliance_tag.startsWith('warn:') ? 'warned' : 'logged')
    return json({
      detections: hits.slice(offset, offset + limit).map(e => ({ id: e.id, agent_id: e.agent_id, type: e.pii_detected[0].pii_type, count: e.pii_detected.length, timestamp: e.timestamp, action: action(e), model: e.model, details: e.pii_detected })),
      total: hits.length, types, summary: { total_events_with_pii: hits.length, total_pii_instances: sum(Object.entries(types).map(([, v]) => ({ v })), o => o.v), unique_types: Object.keys(types).length },
      total_detections: sum(Object.entries(types).map(([, v]) => ({ v })), o => o.v),
      masked_count: hits.filter(e => e.compliance_tag.startsWith('block:')).length,
      pattern_counts: Object.entries(types).map(([pattern, count]) => ({ pattern, count, color: colors[pattern] ?? '#64748b' })).sort((x, y) => y.count - x.count),
      recent_events: hits.slice(0, 20).map(e => ({ time: e.timestamp, agent: e.agent_id, pattern: e.pii_detected[0].pii_type, action: e.compliance_tag.startsWith('block:') ? 'masked' : 'detected', model: e.model ?? 'n/a' })),
    })
  }

  // Policies (mirrors config/policies.example.yaml)
  if (a === 'policies') {
    if (b === 'reload' && m === 'POST') return json({ status: 'ok', message: 'Demo: policies are read-only here. Run the real proxy to reload YAML rules.' })
    const policies = [
      { id: 'pii-detection', name: 'PII Detection', enabled: true, rules_count: 1, action: 'alert', description: 'PII Detection with 1 rule(s)' },
      { id: 'budget-cap', name: 'Daily Budget Cap', enabled: true, rules_count: 1, action: 'block', description: 'Daily Budget Cap with 1 rule(s)' },
    ]
    const tags = new Map<string, number>()
    for (const e of d.events) if (e.compliance_tag !== 'pass:all') tags.set(e.compliance_tag, (tags.get(e.compliance_tag) ?? 0) + 1)
    return json({ policies, total: policies.length, enforcement_stats: [...tags].map(([tag, count]) => ({ tag, count })).sort((x, y) => y.count - x.count),
      total_rules: 2, enabled_rules: 2, policy_enabled: true, pii_masking_enabled: false,
      rules: [
        { id: 'pii-detection', name: 'PII Detection', enabled: true, conditions: 'scan_for: email, phone, ssn, credit_card, ip_address', action: 'alert', priority: 1 },
        { id: 'budget-cap', name: 'Daily Budget Cap', enabled: true, conditions: 'agent_class: * | daily_cost_limit_usd: 100.0 | daily_token_limit: 1000000', action: 'block', priority: 2 },
      ] })
  }

  // Kill switch
  if (a === 'kill-switch') {
    if (b === 'status') {
      const blocked = d.agents.filter(x => x.status === 'blocked')
      return json({ killed_agents: blocked.map(x => ({ agent_id: x.id, name: x.name, status: x.status, last_seen_at: x.last_seen_at, blocked_at: x.updated_at })), killed_count: blocked.length, recent_kills: d.killed.map(s => ({ session_id: s.session_id, agent_id: s.agent_id, killed_at: s.killed_at, timestamp: s.killed_at, killed_by: s.killed_by, reason: s.kill_reason, method: 'circuit_breaker' })) })
    }
    if (b === 'history') {
      const blocks = d.events.filter(e => e.compliance_tag.startsWith('block:')).slice(0, 50)
      const today = new Date().toISOString().slice(0, 10)
      return json({ killed_today: blocks.filter(e => e.timestamp.slice(0, 10) === today).length, circuit_breakers_triggered: 0, events: blocks.map(e => ({ id: e.id, agent_id: e.agent_id, timestamp: e.timestamp, compliance_tag: e.compliance_tag, event_kind: e.event_kind, risk_score: e.risk_score, time: e.timestamp, agent_name: d.agents.find(x => x.id === e.agent_id)?.name ?? e.agent_id, killed_by: 'policy-engine', reason: e.error_message ?? e.compliance_tag, method: 'circuit_breaker' })) })
    }
    if ((b === 'kill' || b === 'revive') && m === 'POST') {
      const p = await body()
      const ag = d.agents.find(x => x.id === p.agent_id)
      if (!ag) return err(404, 'agent not found', { agent_id: p.agent_id })
      ag.status = b === 'kill' ? 'blocked' : 'active'
      return json({ status: b === 'kill' ? 'killed' : 'revived', agent_id: ag.id, note: 'demo: not persisted across function instances' })
    }
  }

  // Sessions
  if (a === 'sessions') {
    if (b) { const rec = sessionRecording(b); return rec ? json(rec) : err(404, 'session not found', { session_id: b }) }
    const ag = q.get('agent_id'); const st = q.get('status')
    const rows = d.sessions.filter(s => (!ag || s.agent_id === ag) && (!st || s.status === st))
    const limit = num(q, 'limit', 50, 1, 200); const offset = num(q, 'offset', 0, 0, 1e9)
    return json({ data: rows.slice(offset, offset + limit), total: rows.length, limit, offset })
  }

  // Demo helper: where to start clicking.
  if (a === 'demo') return json({ mode: 'demo', note: 'Seeded sample data. The real Rust proxy/API is not running.', sample_session_id: d.sessions[0]?.session_id, sample_trace_id: d.traces[0]?.trace_id, agents: AGENT_DEFS.map(x => x.id), events: d.events.length })

  return err(404, 'not found', { path })
}

async function any(req: Request): Promise<Response> {
  try { return await handle(req) } catch (e) { console.error('[demo-api] error', e); return err(500, 'internal error', { detail: String((e as Error)?.message ?? e) }) }
}
export const GET = any
export const POST = any
export const PUT = any
export const DELETE = any
export const OPTIONS = any
