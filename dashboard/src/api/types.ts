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

// ── Enterprise / Platform types ──────────────────────────────────────────────

export interface PlatformHealth {
  status: string
  version: string
  license_tier: string
  max_agents: number
  policy_enabled: boolean
  pii_masking_enabled: boolean
  compliance_enabled: boolean
  a2a_identity_enabled: boolean
  mtls_enabled: boolean
  audit_trail_enabled: boolean
  budget_tracking_enabled: boolean
}

export interface LicenseInfo {
  tier: string
  max_agents: number
  policy_enabled: boolean
  pii_masking_enabled: boolean
  compliance_enabled: boolean
  a2a_identity_enabled: boolean
  retention_days: number
}

export interface PolicySummary {
  total_rules: number
  enabled_rules: number
  policy_enabled: boolean
  pii_masking_enabled: boolean
  rules?: PolicyRule[]
}

export interface PolicyRule {
  id: string
  name: string
  enabled: boolean
  conditions: string
  action: string
  priority: number
}

export interface TenantInfo {
  id: string
  name: string
  max_agents: number
  created_at?: string
}

export interface ComplianceReport {
  framework: string
  generated_at: string
  overall_score: number
  controls: ComplianceControl[]
}

export interface ComplianceControl {
  id: string
  name: string
  description: string
  status: 'pass' | 'fail' | 'partial' | 'not_applicable'
  evidence?: string
}

export interface SessionRecording {
  session_id: string
  agent_id: string
  started_at: string
  ended_at?: string
  event_count: number
  integrity_hash: string
  events: SessionEvent[]
}

export interface SessionEvent {
  timestamp: string
  kind: string
  model?: string
  input_tokens?: number
  output_tokens?: number
  latency_ms?: number
  status_code?: number
  error_message?: string
}

export interface BudgetStatus {
  global_tokens_used: number
  global_tokens_limit?: number
  global_cost_used: number
  global_cost_limit?: number
  reset_days?: number
  agents: AgentBudget[]
}

// ── Risk Overview ─────────────────────────────────────────────────────────────

export interface RiskOverview {
  risk_score: number
  risk_label: string
  alerts: RiskAlert[]
  trend: { day: string; score: number }[]
  stats: {
    total_alerts: number
    critical: number
    high: number
    medium: number
    low: number
    policy_violations_24h: number
    pii_detections_24h: number
  }
}

export interface RiskAlert {
  id: string
  severity: 'critical' | 'high' | 'medium' | 'low'
  message: string
  agent: string
  timestamp: string
}

// ── PII Activity ──────────────────────────────────────────────────────────────

export interface PiiActivity {
  total_detections: number
  masked_count: number
  pattern_counts: { pattern: string; count: number; color: string }[]
  recent_events: PiiEvent[]
}

export interface PiiEvent {
  time: string
  agent: string
  pattern: string
  action: 'masked' | 'detected'
  model: string
}

// ── Kill Switch ───────────────────────────────────────────────────────────────

export interface KillSwitchHistory {
  killed_today: number
  circuit_breakers_triggered: number
  events: KillEvent[]
}

export interface KillEvent {
  time: string
  agent_name: string
  agent_id: string
  killed_by: string
  reason: string
  method: 'manual' | 'budget' | 'circuit_breaker'
}

export interface AgentBudget {
  agent_id: string
  name?: string
  tokens_used: number
  tokens_limit?: number
  cost_used: number
  cost_limit?: number
}
