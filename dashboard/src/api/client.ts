// dashboard/src/api/client.ts
import type {
  AgentEvent, Agent, CostSummary, CostBreakdown,
  HealthResponse, Report, ReportType, GenerateReportRequest,
  SystemConfig, PaginatedResponse, EventFilters,
} from './types'

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message)
    this.name = 'ApiError'
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    headers: { 'Content-Type': 'application/json', ...init?.headers },
    ...init,
  })
  if (!res.ok) {
    const text = await res.text().catch(() => res.statusText)
    throw new ApiError(res.status, text)
  }
  return res.json() as Promise<T>
}

function buildParams(filters: Record<string, string | number | boolean | undefined>): string {
  const p = new URLSearchParams()
  for (const [k, v] of Object.entries(filters)) {
    if (v !== undefined && v !== null) p.set(k, String(v))
  }
  const s = p.toString()
  return s ? `?${s}` : ''
}

// Health
export const getHealth = () => request<HealthResponse>('/health')

// Events
export const getEvents = (filters: EventFilters = {}) =>
  request<PaginatedResponse<AgentEvent>>(`/api/v1/events${buildParams(filters as Record<string, string | number | boolean | undefined>)}`)
export const getEvent = (id: string) =>
  request<AgentEvent>(`/api/v1/events/${id}`)
export const getSessionEvents = (sessionId: string) =>
  request<PaginatedResponse<AgentEvent>>(`/api/v1/events/sessions/${sessionId}`)

// Agents
export const getAgents = () =>
  request<PaginatedResponse<Agent>>('/api/v1/agents')
export const getAgent = (id: string) =>
  request<Agent>(`/api/v1/agents/${id}`)
export const updateAgent = (id: string, body: Partial<Pick<Agent, 'name' | 'status'>>) =>
  request<Agent>(`/api/v1/agents/${id}`, { method: 'PUT', body: JSON.stringify(body) })
export const retireAgent = (id: string) =>
  request<void>(`/api/v1/agents/${id}/retire`, { method: 'POST' })
export const getAgentEvents = (id: string, filters: EventFilters = {}) =>
  request<PaginatedResponse<AgentEvent>>(`/api/v1/agents/${id}/events${buildParams(filters as Record<string, string | number | boolean | undefined>)}`)

// Costs
export const getCostSummary = () =>
  request<CostSummary>('/api/v1/costs/summary')
export const getCostBreakdown = () =>
  request<CostBreakdown>('/api/v1/costs/breakdown')

// Reports
export const getReportTypes = () =>
  request<PaginatedResponse<ReportType>>('/api/v1/reports/types')
export const getReports = () =>
  request<PaginatedResponse<Report>>('/api/v1/reports')
export const generateReport = (body: GenerateReportRequest) =>
  request<Report>('/api/v1/reports/generate', { method: 'POST', body: JSON.stringify(body) })

// Config
export const getConfig = () =>
  request<SystemConfig>('/api/v1/config')
