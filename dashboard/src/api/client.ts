/**
 * Scout API client — typed fetch wrapper pointing to localhost:8080.
 *
 * In development, Vite proxies /api/* and /health to http://localhost:8080.
 * In production (Docker), nginx proxies /api/ to the API container.
 */

import type {
  AgentEvent,
  Agent,
  AgentsResponse,
  EventsResponse,
  CostSummary,
  CostBreakdown,
  HealthResponse,
  AppConfig,
  ReportType,
  Report,
  EventFilters,
  AgentFilters,
  CostParams,
} from './types'

// In dev, Vite proxy handles /api -> localhost:8080.
// In prod, set VITE_API_URL or rely on nginx same-origin proxy.
const API_BASE = import.meta.env.VITE_API_URL ?? ''

// API key: env var > localStorage > dev default
function getApiKey(): string {
  return (
    (import.meta.env.VITE_API_KEY as string | undefined) ||
    localStorage.getItem('govrix_api_key') ||
    'govrix-local-dev'
  )
}

// ── HTTP helpers ──────────────────────────────────────────────────────────────

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    message: string,
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const url = `${API_BASE}${path}`
  const response = await fetch(url, {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${getApiKey()}`,
      ...init?.headers,
    },
    ...init,
  })

  if (!response.ok) {
    const body = await response.text()
    throw new ApiError(response.status, body || `HTTP ${response.status}`)
  }

  return response.json() as Promise<T>
}

function buildQuery(params: Record<string, string | number | boolean | undefined>): string {
  const qs = new URLSearchParams()
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== null && v !== '') {
      qs.set(k, String(v))
    }
  }
  const s = qs.toString()
  return s ? `?${s}` : ''
}

// ── Events ────────────────────────────────────────────────────────────────────

export async function fetchEvents(filters?: EventFilters): Promise<EventsResponse> {
  const q = buildQuery({
    agent_id: filters?.agent_id,
    kind: filters?.kind,
    protocol: filters?.protocol,
    model: filters?.model,
    since: filters?.since,
    until: filters?.until,
    limit: filters?.limit ?? 50,
    page: filters?.page,
  })
  const raw = await apiFetch<{ data: AgentEvent[]; total: number }>(`/api/v1/events${q}`)
  return { events: raw.data ?? [], total: raw.total ?? 0 }
}

export async function fetchEvent(id: string): Promise<AgentEvent> {
  const raw = await apiFetch<{ data: AgentEvent } | AgentEvent>(`/api/v1/events/${id}`)
  return ('data' in raw ? raw.data : raw) as AgentEvent
}

export function fetchSessionEvents(sessionId: string): Promise<{ session_id: string; events: AgentEvent[] }> {
  return apiFetch<{ session_id: string; events: AgentEvent[] }>(`/api/v1/events/sessions/${sessionId}`)
}

// ── Agents ────────────────────────────────────────────────────────────────────

export async function fetchAgents(filters?: AgentFilters): Promise<AgentsResponse> {
  const q = buildQuery({
    status: filters?.status,
    search: filters?.search,
    limit: filters?.limit ?? 100,
  })
  const raw = await apiFetch<{ data: Record<string, unknown>[]; total: number }>(`/api/v1/agents${q}`)
  const agents: Agent[] = (raw.data ?? []).map(a => ({
    ...a,
    total_requests: (a.total_requests as number) ?? 0,
    total_tokens: (a.total_tokens as number) ?? ((a.total_tokens_in as number ?? 0) + (a.total_tokens_out as number ?? 0)),
    total_cost_usd: (a.total_cost_usd as number) ?? 0,
    error_count: (a.error_count as number) ?? 0,
  })) as Agent[]
  return { agents, total: raw.total ?? 0 }
}

export function fetchAgent(id: string): Promise<Agent> {
  return apiFetch<Agent>(`/api/v1/agents/${id}`)
}

export function updateAgent(id: string, data: Partial<Pick<Agent, 'name' | 'description' | 'labels'>>): Promise<Agent> {
  return apiFetch<Agent>(`/api/v1/agents/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  })
}

export function retireAgent(id: string): Promise<void> {
  return apiFetch<void>(`/api/v1/agents/${id}/retire`, { method: 'POST' })
}

// ── Costs ─────────────────────────────────────────────────────────────────────

export async function fetchCostSummary(params?: CostParams): Promise<CostSummary> {
  const q = buildQuery({
    days: params?.days,
    since: params?.since,
    until: params?.until,
    agent_id: params?.agent_id,
  })
  const raw = await apiFetch<{ data: { total_cost_usd: number; total_requests: number; total_input_tokens: number; total_output_tokens: number; avg_latency_ms?: number; from?: string; to?: string } }>(`/api/v1/costs/summary${q}`)
  const d = raw.data
  return {
    total_cost_usd: d.total_cost_usd ?? 0,
    total_requests: d.total_requests ?? 0,
    total_tokens: (d.total_input_tokens ?? 0) + (d.total_output_tokens ?? 0),
    avg_latency_ms: d.avg_latency_ms ?? undefined,
    period_start: d.from,
    period_end: d.to,
  }
}

export async function fetchCostBreakdown(params?: CostParams): Promise<CostBreakdown> {
  const q = buildQuery({
    days: params?.days,
    since: params?.since,
    until: params?.until,
  })
  type BreakdownRow = { group_key: string; request_count: number; total_input_tokens: number; total_output_tokens: number; total_cost_usd: number }
  const [modelRaw, agentRaw] = await Promise.all([
    apiFetch<{ data: BreakdownRow[] }>(`/api/v1/costs/breakdown?group_by=model${q ? q.replace('?', '&') : ''}`),
    apiFetch<{ data: BreakdownRow[] }>(`/api/v1/costs/breakdown?group_by=agent${q ? q.replace('?', '&') : ''}`),
  ])
  const mapRow = (r: BreakdownRow) => ({
    group: r.group_key,
    requests: r.request_count,
    tokens: (r.total_input_tokens ?? 0) + (r.total_output_tokens ?? 0),
    cost_usd: r.total_cost_usd,
  })
  return {
    by_model: (modelRaw.data ?? []).map(mapRow),
    by_agent: (agentRaw.data ?? []).map(mapRow),
    by_protocol: [],
    daily: [],
  }
}

// ── Reports ───────────────────────────────────────────────────────────────────

export async function fetchReportTypes(): Promise<{ types: ReportType[] }> {
  const raw = await apiFetch<{ data: ReportType[]; total: number }>('/api/v1/reports/types')
  return { types: raw.data ?? [] }
}

export function generateReport(reportType: string): Promise<Report> {
  return apiFetch<Report>('/api/v1/reports/generate', {
    method: 'POST',
    body: JSON.stringify({ report_type: reportType }),
  })
}

export async function fetchReports(): Promise<{ reports: Report[] }> {
  const raw = await apiFetch<{ data: Report[]; total: number }>('/api/v1/reports')
  return { reports: raw.data ?? [] }
}

// ── Health & Config ───────────────────────────────────────────────────────────

export function fetchHealth(): Promise<HealthResponse> {
  return apiFetch<HealthResponse>('/health')
}

export async function fetchConfig(): Promise<AppConfig> {
  const raw = await apiFetch<{ data: AppConfig } | AppConfig>('/api/v1/config')
  return ('data' in raw ? raw.data : raw) as AppConfig
}
