/**
 * Trace & span types and API client functions.
 * Mirrors the Rust TraceSpan / Trace models.
 */

// ── Types ─────────────────────────────────────────────────────────────────────

export type TraceStatus = 'running' | 'completed' | 'stopped' | 'failed'

export interface TraceSpan {
  id: string
  trace_id: string
  parent_span_id?: string
  agent_id: string
  event_kind: string
  tool_name?: string
  mcp_server?: string
  model?: string
  started_at: string
  ended_at?: string
  latency_ms?: number
  input_tokens?: number
  output_tokens?: number
  total_tokens?: number
  cost_usd?: number
  risk_score?: number
  tool_args?: Record<string, unknown>
  tool_result?: Record<string, unknown>
  error_message?: string
  status_code?: number
  // computed client-side
  depth?: number
}

export interface Trace {
  trace_id: string
  root_agent: string
  status: TraceStatus
  started_at: string
  ended_at?: string
  duration_ms?: number
  total_cost_usd?: number
  peak_risk_score?: number
  span_count?: number
  spans?: TraceSpan[]
}

export interface TracesResponse {
  traces: Trace[]
  total: number
}

export interface TraceFilters {
  agent_id?: string
  status?: TraceStatus | 'all'
  since?: string
  until?: string
  limit?: number
  offset?: number
}

// ── Span tree helpers ─────────────────────────────────────────────────────────

export interface SpanTreeNode extends TraceSpan {
  depth: number
  children: SpanTreeNode[]
}

export function buildSpanTree(spans: TraceSpan[]): SpanTreeNode[] {
  // Map span id → children
  const childMap = new Map<string | undefined, TraceSpan[]>()
  for (const span of spans) {
    const parent = span.parent_span_id
    if (!childMap.has(parent)) childMap.set(parent, [])
    childMap.get(parent)!.push(span)
  }

  // Sort children by timestamp
  const sortByTime = (arr: TraceSpan[]) =>
    [...arr].sort((a, b) => new Date(a.started_at).getTime() - new Date(b.started_at).getTime())

  function buildNode(span: TraceSpan, depth: number): SpanTreeNode {
    const rawChildren = childMap.get(span.id) ?? []
    return {
      ...span,
      depth,
      children: sortByTime(rawChildren).map(c => buildNode(c, depth + 1)),
    }
  }

  // Roots = spans with no parent OR parent not in span set
  const spanIds = new Set(spans.map(s => s.id))
  const roots = sortByTime(
    spans.filter(s => !s.parent_span_id || !spanIds.has(s.parent_span_id))
  )
  return roots.map(r => buildNode(r, 0))
}

export function flattenTree(nodes: SpanTreeNode[]): SpanTreeNode[] {
  const result: SpanTreeNode[] = []
  function walk(arr: SpanTreeNode[]) {
    for (const node of arr) {
      result.push(node)
      walk(node.children)
    }
  }
  walk(nodes)
  return result
}

// ── API client ────────────────────────────────────────────────────────────────

const API_BASE = import.meta.env.VITE_API_URL ?? ''

function getApiKey(): string {
  return (
    (import.meta.env.VITE_API_KEY as string | undefined) ||
    localStorage.getItem('govrix_api_key') ||
    'govrix-local-dev'
  )
}

async function traceFetch<T>(path: string): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      'Content-Type': 'application/json',
      Authorization: `Bearer ${getApiKey()}`,
    },
  })
  if (!response.ok) {
    const body = await response.text()
    throw new Error(body || `HTTP ${response.status}`)
  }
  return response.json() as Promise<T>
}

function buildQuery(params: Record<string, string | number | boolean | undefined>): string {
  const qs = new URLSearchParams()
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined && v !== null && v !== '' && v !== 'all') {
      qs.set(k, String(v))
    }
  }
  const s = qs.toString()
  return s ? `?${s}` : ''
}

export async function fetchTraces(filters?: TraceFilters): Promise<TracesResponse> {
  const q = buildQuery({
    agent_id: filters?.agent_id,
    status: filters?.status,
    since: filters?.since,
    until: filters?.until,
    limit: filters?.limit ?? 25,
    offset: filters?.offset ?? 0,
  })
  try {
    const raw = await traceFetch<{ data: Trace[]; total: number }>(`/api/v1/traces${q}`)
    return { traces: raw.data ?? [], total: raw.total ?? 0 }
  } catch {
    // Return empty if endpoint doesn't exist yet
    return { traces: [], total: 0 }
  }
}

export async function fetchTrace(traceId: string): Promise<Trace | null> {
  try {
    const raw = await traceFetch<{ data: Trace } | Trace>(`/api/v1/traces/${traceId}`)
    return 'data' in raw ? raw.data : raw
  } catch {
    return null
  }
}
