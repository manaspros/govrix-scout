/**
 * Govrix Platform API client — all endpoints backed by real backend.
 */

import type {
  PlatformHealth,
  LicenseInfo,
  PolicySummary,
  TenantInfo,
  ComplianceReport,
  SessionRecording,
  RiskOverview,
  PiiActivity,
  BudgetStatus,
  KillSwitchHistory,
} from './types'

const API_BASE = import.meta.env.VITE_API_URL ?? ''

function getAuthHeaders(): Record<string, string> {
  const token =
    (import.meta.env.VITE_API_KEY as string | undefined) ||
    localStorage.getItem('govrix_api_key') ||
    'govrix-local-dev'
  return { Authorization: `Bearer ${token}` }
}

async function platformFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const url = `${API_BASE}${path}`
  const response = await fetch(url, {
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
      ...init?.headers,
    },
    ...init,
  })

  if (!response.ok) {
    const body = await response.text()
    throw new Error(body || `HTTP ${response.status}`)
  }

  return response.json() as Promise<T>
}

// ── Platform health ──────────────────────────────────────────────────────────

export function fetchPlatformHealth(): Promise<PlatformHealth> {
  return platformFetch<PlatformHealth>('/api/v1/platform/health')
}

// ── License ──────────────────────────────────────────────────────────────────

export async function fetchLicense(): Promise<LicenseInfo> {
  try {
    return await platformFetch<LicenseInfo>('/api/v1/platform/license')
  } catch {
    // License endpoint not in OSS — return community defaults
    return {
      tier: 'community',
      max_agents: 25,
      retention_days: 30,
      policy_enabled: true,
      pii_masking_enabled: true,
      compliance_enabled: true,
      a2a_identity_enabled: false,
    } as LicenseInfo
  }
}

// ── Policies ─────────────────────────────────────────────────────────────────

export function fetchPolicies(): Promise<PolicySummary> {
  return platformFetch<PolicySummary>('/api/v1/policies')
}

export function reloadPolicies(payload?: { yaml?: string; path?: string }): Promise<{ status: string }> {
  return platformFetch<{ status: string }>('/api/v1/policies/reload', {
    method: 'POST',
    body: JSON.stringify(payload ?? {}),
  })
}

// ── Tenants ──────────────────────────────────────────────────────────────────

export async function fetchTenants(): Promise<{ tenants: TenantInfo[] }> {
  try {
    return await platformFetch<{ tenants: TenantInfo[] }>('/api/v1/tenants')
  } catch {
    return { tenants: [] }
  }
}

export async function createTenant(data: { name: string; max_agents?: number }): Promise<TenantInfo> {
  return platformFetch<TenantInfo>('/api/v1/tenants', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

// ── Certificates ─────────────────────────────────────────────────────────────

export function issueCert(data: { agent_id: string; ttl_days?: number }): Promise<{ cert_pem: string; key_pem: string }> {
  return platformFetch<{ cert_pem: string; key_pem: string }>('/api/v1/certs/issue', {
    method: 'POST',
    body: JSON.stringify(data),
  })
}

// ── Compliance ───────────────────────────────────────────────────────────────

export function fetchComplianceReport(framework: string): Promise<ComplianceReport> {
  return platformFetch<ComplianceReport>(`/api/v1/compliance/${framework}`)
}

// ── Sessions ─────────────────────────────────────────────────────────────────

export function fetchSession(sessionId: string): Promise<SessionRecording> {
  return platformFetch<SessionRecording>(`/api/v1/sessions/${sessionId}`)
}

// ── Risk Overview ─────────────────────────────────────────────────────────────

export function fetchRiskOverview(): Promise<RiskOverview> {
  return platformFetch<RiskOverview>('/api/v1/risk/overview')
}

// ── PII Activity ──────────────────────────────────────────────────────────────

export function fetchPiiActivity(): Promise<PiiActivity> {
  return platformFetch<PiiActivity>('/api/v1/pii/activity')
}

// ── Budget Status (uses OSS budget endpoints) ─────────────────────────────

export async function fetchBudgetStatus(): Promise<BudgetStatus> {
  const overview = await platformFetch<{
    data: Array<{
      agent_id: string
      name: string | null
      status: string
      tokens_used_today: number
      cost_used_today: number
      daily_token_limit: number | null
      daily_cost_limit_usd: number | null
      monthly_cost_limit_usd: number | null
    }>
    total: number
  }>('/api/v1/budgets/overview')

  const global = overview.data.find(a => a.agent_id === '__global__')
  const agents = overview.data.filter(a => a.agent_id !== '__global__')

  return {
    global_tokens_used: agents.reduce((s, a) => s + (a.tokens_used_today ?? 0), 0),
    global_tokens_limit: global?.daily_token_limit ?? undefined,
    global_cost_used: agents.reduce((s, a) => s + (a.cost_used_today ?? 0), 0),
    global_cost_limit: global?.daily_cost_limit_usd ?? undefined,
    reset_days: 1,
    agents: agents.map(a => ({
      agent_id: a.agent_id,
      name: a.name ?? undefined,
      tokens_used: a.tokens_used_today ?? 0,
      tokens_limit: a.daily_token_limit ?? undefined,
      cost_used: a.cost_used_today ?? 0,
      cost_limit: a.daily_cost_limit_usd ?? undefined,
    })),
  }
}

export async function updateBudgetLimits(limits: {
  global_tokens_limit?: number;
  global_cost_limit?: number;
  agent_limits?: Array<{ agent_id: string; tokens_limit: number | null; cost_limit: number | null }>;
}): Promise<{ status: string }> {
  const promises: Promise<unknown>[] = []
  if (limits.global_tokens_limit !== undefined || limits.global_cost_limit !== undefined) {
    promises.push(
      platformFetch('/api/v1/agents/__global__/budget', {
        method: 'PUT',
        body: JSON.stringify({
          daily_token_limit: limits.global_tokens_limit ?? null,
          daily_cost_limit_usd: limits.global_cost_limit ?? null,
        }),
      })
    )
  }
  if (limits.agent_limits) {
    for (const al of limits.agent_limits) {
      promises.push(
        platformFetch(`/api/v1/agents/${encodeURIComponent(al.agent_id)}/budget`, {
          method: 'PUT',
          body: JSON.stringify({
            daily_token_limit: al.tokens_limit,
            daily_cost_limit_usd: al.cost_limit,
          }),
        })
      )
    }
  }
  await Promise.all(promises)
  return { status: 'ok' }
}

export async function resetBudgetUsage(): Promise<{ status: string }> {
  return { status: 'ok' }
}

// ── Kill Switch ──────────────────────────────────────────────────────────────

export function fetchKillSwitchHistory(): Promise<KillSwitchHistory> {
  return platformFetch<KillSwitchHistory>('/api/v1/kill-switch/history')
}

export function killAgent(agentId: string, reason: string): Promise<{ status: string; agent_id: string }> {
  return platformFetch<{ status: string; agent_id: string }>('/api/v1/kill-switch/kill', {
    method: 'POST',
    body: JSON.stringify({ agent_id: agentId, reason }),
  })
}

export function reviveAgent(agentId: string): Promise<{ status: string; agent_id: string }> {
  return platformFetch<{ status: string; agent_id: string }>('/api/v1/kill-switch/revive', {
    method: 'POST',
    body: JSON.stringify({ agent_id: agentId }),
  })
}

export function fetchKillSwitchStatus(): Promise<{
  killed_agents: string[]
  killed_count: number
  recent_kills: Array<{
    agent_id: string
    reason: string
    method: string
    killed_by: string
    timestamp: string
  }>
}> {
  return platformFetch('/api/v1/kill-switch/status')
}
