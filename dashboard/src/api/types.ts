// dashboard/src/api/types.ts

export interface HealthResponse {
  status: 'ok' | 'degraded' | 'error'
  version: string
  uptime_secs: number
}

export interface AgentEvent {
  id: string
  session_id: string
  agent_id: string
  timestamp: string
  kind: string
  protocol: string
  model: string | null
  provider: string | null
  input_tokens: number | null
  output_tokens: number | null
  cost_usd: number | null
  latency_ms: number | null
  status_code: number | null
  pii_detected: boolean
  compliance_tag: string
  lineage_hash: string
  request_body: string | null
  response_body: string | null
}

export interface Agent {
  id: string
  name: string
  status: 'active' | 'retired' | 'blocked'
  first_seen: string
  last_seen: string
  total_requests: number
  total_cost_usd: number
  total_input_tokens: number
  total_output_tokens: number
}

export interface CostSummary {
  total_cost_usd: number
  total_requests: number
  total_input_tokens: number
  total_output_tokens: number
  avg_cost_per_request: number
  period_start: string
  period_end: string
}

export interface CostBucket {
  label: string
  cost_usd: number
  requests: number
  input_tokens: number
  output_tokens: number
}

export interface CostBreakdown {
  by_model: CostBucket[]
  by_agent: CostBucket[]
  by_provider: CostBucket[]
}

export interface ReportType {
  id: string
  name: string
  description: string
}

export interface Report {
  id: string
  report_type: string
  status: 'pending' | 'complete' | 'failed'
  created_at: string
  download_url: string | null
}

export interface GenerateReportRequest {
  report_type: string
  format: 'pdf' | 'json' | 'csv'
  start_date?: string
  end_date?: string
}

export interface SystemConfig {
  proxy_port: number
  management_port: number
  max_agents: number
  retention_days: number
  pii_detection_enabled: boolean
  budget_enforcement_enabled: boolean
}

export interface PaginatedResponse<T> {
  data: T[]
  total: number
}

export interface EventFilters {
  agent_id?: string
  session_id?: string
  kind?: string
  limit?: number
  offset?: number
}
