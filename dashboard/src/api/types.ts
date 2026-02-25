/**
 * TypeScript types matching the Scout Rust models.
 * These types correspond to the canonical DB schemas in agentmesh-schemas skill.
 */

// ── Core domain types ─────────────────────────────────────────────────────────

export interface AgentEvent {
  id: string
  session_id: string
  agent_id: string
  kind: string
  protocol: string
  upstream_target: string
  timestamp: string
  model?: string
  input_tokens?: number
  output_tokens?: number
  total_tokens?: number
  cost_usd?: number
  latency_ms?: number
  status_code?: number
  finish_reason?: string
  payload: Record<string, unknown>
  raw_size_bytes?: number
  tags: Record<string, unknown>
  error_message?: string
  created_at?: string
}

export interface Agent {
  id: string
  name?: string
  description?: string
  framework?: string
  status: AgentStatus
  labels: Record<string, unknown>
  total_requests: number
  total_tokens: number
  total_cost_usd: number
  last_model_used?: string
  last_seen_at?: string
  last_error_at?: string
  error_count: number
  source_ip?: string
  first_seen_at?: string
  created_at?: string
  updated_at?: string
}

export type AgentStatus = 'active' | 'idle' | 'error' | 'blocked' | 'retired'

export interface CostBucket {
  timestamp: string
  requests: number
  tokens: number
  cost_usd: number
}

export interface CostBreakdownGroup {
  group: string
  requests: number
  tokens: number
  cost_usd: number
}

export interface CostSummary {
  total_cost_usd: number
  total_requests: number
  total_tokens: number
  avg_latency_ms?: number
  period_start?: string
  period_end?: string
}

export interface CostBreakdown {
  by_agent: CostBreakdownGroup[]
  by_model: CostBreakdownGroup[]
  by_protocol: CostBreakdownGroup[]
  daily: CostBucket[]
}

// ── Health & config ───────────────────────────────────────────────────────────

export interface HealthResponse {
  status: 'ok' | 'degraded' | 'error'
  version: string
  db?: string
  proxy?: string
  uptime_secs?: number
}

export interface AppConfig {
  proxy_port?: number
  api_port?: number
  database_url?: string
  log_level?: string
  max_body_size_bytes?: number
  event_retention_days?: number
  agent_soft_limit?: number
  [key: string]: unknown
}

// ── Report types ──────────────────────────────────────────────────────────────

export interface ReportType {
  id: string
  name: string
  description: string
  format: 'pdf' | 'json' | 'csv'
}

export interface Report {
  id: string
  report_type: string
  status: 'pending' | 'generating' | 'ready' | 'error'
  created_at: string
  download_url?: string
  error_message?: string
}

// ── List responses ────────────────────────────────────────────────────────────

export interface EventsResponse {
  events: AgentEvent[]
  total: number
  page?: number
  per_page?: number
}

export interface AgentsResponse {
  agents: Agent[]
  total: number
}

// ── Filter params ─────────────────────────────────────────────────────────────

export interface EventFilters {
  agent_id?: string
  kind?: string
  protocol?: string
  model?: string
  since?: string
  until?: string
  limit?: number
  page?: number
}

export interface AgentFilters {
  status?: AgentStatus
  search?: string
  limit?: number
}

export interface CostParams {
  days?: number
  since?: string
  until?: string
  agent_id?: string
}
