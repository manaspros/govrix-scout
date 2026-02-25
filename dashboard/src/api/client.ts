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

export function fetchEvents(filters?: EventFilters): Promise<EventsResponse> {
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
  return apiFetch<EventsResponse>(`/api/v1/events${q}`)
}

export function fetchEvent(id: string): Promise<AgentEvent> {
  return apiFetch<AgentEvent>(`/api/v1/events/${id}`)
}

export function fetchSessionEvents(sessionId: string): Promise<{ session_id: string; events: AgentEvent[] }> {
  return apiFetch<{ session_id: string; events: AgentEvent[] }>(`/api/v1/events/sessions/${sessionId}`)
}

// ── Agents ────────────────────────────────────────────────────────────────────

export function fetchAgents(filters?: AgentFilters): Promise<AgentsResponse> {
  const q = buildQuery({
    status: filters?.status,
    search: filters?.search,
    limit: filters?.limit ?? 100,
  })
  return apiFetch<AgentsResponse>(`/api/v1/agents${q}`)
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

export function fetchCostSummary(params?: CostParams): Promise<CostSummary> {
  const q = buildQuery({
    days: params?.days,
    since: params?.since,
    until: params?.until,
    agent_id: params?.agent_id,
  })
  return apiFetch<CostSummary>(`/api/v1/costs/summary${q}`)
}

export function fetchCostBreakdown(params?: CostParams): Promise<CostBreakdown> {
  const q = buildQuery({
    days: params?.days,
    since: params?.since,
    until: params?.until,
  })
  return apiFetch<CostBreakdown>(`/api/v1/costs/breakdown${q}`)
}

// ── Reports ───────────────────────────────────────────────────────────────────

export function fetchReportTypes(): Promise<{ types: ReportType[] }> {
  return apiFetch<{ types: ReportType[] }>('/api/v1/reports/types')
}

export function generateReport(reportType: string): Promise<Report> {
  return apiFetch<Report>('/api/v1/reports/generate', {
    method: 'POST',
    body: JSON.stringify({ report_type: reportType }),
  })
}

export function fetchReports(): Promise<{ reports: Report[] }> {
  return apiFetch<{ reports: Report[] }>('/api/v1/reports')
}

// ── Health & Config ───────────────────────────────────────────────────────────

export function fetchHealth(): Promise<HealthResponse> {
  return apiFetch<HealthResponse>('/health')
}

export function fetchConfig(): Promise<AppConfig> {
  return apiFetch<AppConfig>('/api/v1/config')
}
