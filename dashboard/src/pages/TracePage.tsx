import { useState, useMemo } from 'react'
import { useParams, Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { format, parseISO } from 'date-fns'
import { ChevronDown, ChevronRight, ArrowLeft, Wrench, Brain, Zap, AlertTriangle, GitBranch } from 'lucide-react'
import { clsx } from 'clsx'
import { fetchTrace, buildSpanTree, flattenTree } from '@/api/traces'
import type { SpanTreeNode, Trace } from '@/api/traces'
import { StatusBadge } from '@/components/common/StatusBadge'

// ── Span kind config ──────────────────────────────────────────────────────────

interface KindConfig {
  color: string        // bar fill
  ring: string         // ring class
  icon: React.ComponentType<{ className?: string; style?: React.CSSProperties }>
  label: string
}

const KIND_CONFIG: Record<string, KindConfig> = {
  'llm.request':    { color: '#3b82f6', ring: 'ring-blue-500/40',    icon: Brain,       label: 'LLM Request' },
  'llm.response':   { color: '#60a5fa', ring: 'ring-blue-400/40',    icon: Brain,       label: 'LLM Response' },
  'tool.invoke':    { color: '#f97316', ring: 'ring-orange-500/40',  icon: Wrench,      label: 'Tool Invoke' },
  'tool.result':    { color: '#22c55e', ring: 'ring-green-500/40',   icon: Zap,         label: 'Tool Result' },
  'policy.block':   { color: '#ef4444', ring: 'ring-red-500/40',     icon: AlertTriangle, label: 'Policy Block' },
  'agent.spawn':    { color: '#a855f7', ring: 'ring-purple-500/40',  icon: GitBranch,   label: 'Agent Spawn' },
  'agent.message':  { color: '#8b5cf6', ring: 'ring-violet-500/40', icon: GitBranch,   label: 'Agent Message' },
  'session.start':  { color: '#10b981', ring: 'ring-emerald-500/40', icon: Zap,         label: 'Session Start' },
  'session.end':    { color: '#6b7280', ring: 'ring-slate-500/40',   icon: Zap,         label: 'Session End' },
}

function kindConfig(kind: string): KindConfig {
  return KIND_CONFIG[kind] ?? {
    color: '#64748b',
    ring: 'ring-slate-500/40',
    icon: Zap,
    label: kind,
  }
}

// ── Risk bar ──────────────────────────────────────────────────────────────────

function RiskBar({ score }: { score: number }) {
  const pct = Math.min(100, Math.max(0, score))
  const color = pct < 30 ? '#22c55e' : pct < 70 ? '#eab308' : '#ef4444'
  return (
    <div className="flex items-center gap-2">
      <div className="flex-1 h-1.5 bg-slate-700 rounded-full overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-300"
          style={{ width: `${pct}%`, background: color }}
        />
      </div>
      <span
        className="text-xs tabular-nums w-7 text-right shrink-0"
        style={{ color, fontFamily: 'JetBrains Mono' }}
      >
        {pct}
      </span>
    </div>
  )
}

// ── Span detail (expanded) ────────────────────────────────────────────────────

function SpanDetail({ span }: { span: SpanTreeNode }) {
  const cfg = kindConfig(span.event_kind)
  const Icon = cfg.icon

  return (
    <div
      className="mx-2 mb-2 rounded-xl border overflow-hidden text-xs"
      style={{
        background: 'rgba(6,10,19,0.7)',
        borderColor: `${cfg.color}33`,
      }}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-4 py-3"
        style={{ borderBottom: `1px solid ${cfg.color}22`, background: `${cfg.color}0d` }}
      >
        <div className="flex items-center gap-2.5">
          <Icon className="w-4 h-4 shrink-0" style={{ color: cfg.color }} />
          <span className="font-semibold text-slate-200">
            {span.tool_name ?? span.model ?? cfg.label}
          </span>
          {span.mcp_server && (
            <span className="text-slate-500">· MCP Server: {span.mcp_server}</span>
          )}
        </div>
        <div className="flex items-center gap-4">
          {span.latency_ms != null && (
            <span className="text-slate-400" style={{ fontFamily: 'JetBrains Mono' }}>
              {span.latency_ms}ms
            </span>
          )}
          {span.risk_score != null && span.risk_score > 0 && (
            <div className="flex items-center gap-1.5 min-w-[100px]">
              <span className="text-slate-500">Risk:</span>
              <RiskBar score={span.risk_score} />
            </div>
          )}
        </div>
      </div>

      {/* Metrics row */}
      {(span.input_tokens || span.output_tokens || span.cost_usd) && (
        <div
          className="flex items-center gap-6 px-4 py-2"
          style={{ borderBottom: `1px solid ${cfg.color}15` }}
        >
          {span.input_tokens != null && (
            <div>
              <span className="text-slate-500">In tokens: </span>
              <span className="text-slate-300 tabular-nums">{span.input_tokens.toLocaleString()}</span>
            </div>
          )}
          {span.output_tokens != null && (
            <div>
              <span className="text-slate-500">Out tokens: </span>
              <span className="text-slate-300 tabular-nums">{span.output_tokens.toLocaleString()}</span>
            </div>
          )}
          {span.cost_usd != null && (
            <div>
              <span className="text-slate-500">Cost: </span>
              <span className="text-emerald-400 tabular-nums">${span.cost_usd.toFixed(6)}</span>
            </div>
          )}
          {span.model && (
            <div>
              <span className="text-slate-500">Model: </span>
              <span className="text-slate-300">{span.model}</span>
            </div>
          )}
        </div>
      )}

      {/* Tool args */}
      {span.tool_args && Object.keys(span.tool_args).length > 0 && (
        <div style={{ borderBottom: `1px solid ${cfg.color}15` }}>
          <div className="px-4 py-2 text-[10px] font-semibold text-slate-500 uppercase tracking-wider">
            Arguments
          </div>
          <pre
            className="px-4 pb-3 overflow-x-auto text-slate-300 leading-relaxed"
            style={{ fontFamily: 'JetBrains Mono', fontSize: '11px', maxHeight: '200px' }}
          >
            {JSON.stringify(span.tool_args, null, 2)}
          </pre>
        </div>
      )}

      {/* Tool result */}
      {span.tool_result && Object.keys(span.tool_result).length > 0 && (
        <div>
          <div className="px-4 py-2 text-[10px] font-semibold text-slate-500 uppercase tracking-wider">
            Result
          </div>
          <pre
            className="px-4 pb-3 overflow-x-auto text-slate-300 leading-relaxed"
            style={{ fontFamily: 'JetBrains Mono', fontSize: '11px', maxHeight: '200px' }}
          >
            {JSON.stringify(span.tool_result, null, 2)}
          </pre>
        </div>
      )}

      {/* Error */}
      {span.error_message && (
        <div className="px-4 py-3 bg-red-900/20">
          <div className="text-[10px] font-semibold text-red-500 uppercase tracking-wider mb-1">Error</div>
          <div className="text-red-400">{span.error_message}</div>
        </div>
      )}
    </div>
  )
}

// ── Span row (waterfall row) ──────────────────────────────────────────────────

interface SpanRowProps {
  span: SpanTreeNode
  maxLatency: number
  traceStart: number
  traceEnd: number
  isExpanded: boolean
  onToggle: () => void
}

function SpanRow({ span, maxLatency, traceStart, traceEnd, isExpanded, onToggle }: SpanRowProps) {
  const cfg = kindConfig(span.event_kind)
  const Icon = cfg.icon

  const totalDuration = Math.max(traceEnd - traceStart, 1)
  const spanStart = new Date(span.started_at).getTime() - traceStart
  const spanWidth = Math.max(span.latency_ms ?? 10, 10)

  const leftPct = (spanStart / totalDuration) * 100
  const widthPct = Math.min(((spanWidth / totalDuration) * 100), 100 - leftPct)
  const barWidthPct = maxLatency > 0
    ? Math.max(((span.latency_ms ?? 0) / maxLatency) * 40, 2)
    : 2

  return (
    <div
      className={clsx(
        'border-b transition-colors cursor-pointer',
        isExpanded
          ? 'bg-slate-800/60'
          : 'hover:bg-white/[0.02]',
      )}
      style={{ borderColor: 'rgba(148,163,184,0.06)' }}
    >
      {/* Main row */}
      <div
        className="flex items-center gap-3 px-4 py-2.5"
        onClick={onToggle}
        style={{ paddingLeft: `${16 + span.depth * 20}px` }}
      >
        {/* Expand icon */}
        <button className="shrink-0 text-slate-600 hover:text-slate-400 transition-colors">
          {isExpanded
            ? <ChevronDown className="w-3.5 h-3.5" />
            : <ChevronRight className="w-3.5 h-3.5" />}
        </button>

        {/* Kind icon */}
        <Icon className="w-3.5 h-3.5 shrink-0" style={{ color: cfg.color }} />

        {/* Label */}
        <div className="w-40 shrink-0 min-w-0">
          <div className="text-xs font-medium text-slate-200 truncate">
            {span.tool_name ?? span.model ?? cfg.label}
          </div>
          <div className="text-[10px] text-slate-600 truncate" style={{ fontFamily: 'JetBrains Mono' }}>
            {span.event_kind}
          </div>
        </div>

        {/* Waterfall bar */}
        <div className="flex-1 h-5 relative rounded-sm overflow-hidden" style={{ background: 'rgba(148,163,184,0.05)' }}>
          {/* Position bar relative to trace start */}
          <div
            className="absolute top-1 bottom-1 rounded-sm opacity-80"
            style={{
              left: `${Math.min(leftPct, 95)}%`,
              width: `${Math.max(widthPct, 0.5)}%`,
              background: cfg.color,
              minWidth: '4px',
            }}
          />
        </div>

        {/* Relative bar (proportional to widest span) */}
        <div className="w-24 shrink-0">
          <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
            <div
              className="h-full rounded-full"
              style={{ width: `${barWidthPct}%`, background: cfg.color, opacity: 0.7 }}
            />
          </div>
        </div>

        {/* Latency */}
        <div
          className="w-16 shrink-0 text-right text-xs tabular-nums text-slate-500"
          style={{ fontFamily: 'JetBrains Mono' }}
        >
          {span.latency_ms != null ? `${span.latency_ms}ms` : '—'}
        </div>

        {/* Risk */}
        <div className="w-20 shrink-0">
          {span.risk_score != null && span.risk_score > 0 ? (
            <RiskBar score={span.risk_score} />
          ) : (
            <div className="text-xs text-slate-700 text-center">—</div>
          )}
        </div>

        {/* Time */}
        <div
          className="w-20 shrink-0 text-right text-[10px] text-slate-600 whitespace-nowrap"
          style={{ fontFamily: 'JetBrains Mono' }}
        >
          {format(parseISO(span.started_at), 'HH:mm:ss.SSS')}
        </div>
      </div>

      {/* Expanded detail */}
      {isExpanded && <SpanDetail span={span} />}
    </div>
  )
}

// ── Trace header ──────────────────────────────────────────────────────────────

function TraceHeader({ trace }: { trace: Trace }) {
  const statusColor: Record<string, string> = {
    running: '#eab308',
    completed: '#22c55e',
    stopped: '#6b7280',
    failed: '#ef4444',
  }
  const color = statusColor[trace.status] ?? '#64748b'

  return (
    <div className="glass-card p-5">
      <div className="flex flex-wrap items-start gap-4">
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-3 mb-2">
            <span
              className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold ring-1"
              style={{
                color,
                background: `${color}18`,
                borderColor: `${color}40`,
              }}
            >
              {trace.status === 'running' && (
                <span
                  className="w-1.5 h-1.5 rounded-full animate-pulse"
                  style={{ background: color }}
                />
              )}
              {trace.status}
            </span>
            <span className="text-sm text-slate-400">
              Agent: <span className="text-slate-200 font-medium">{trace.root_agent}</span>
            </span>
          </div>
          <div
            className="text-xs text-slate-600 font-mono truncate"
            title={trace.trace_id}
          >
            {trace.trace_id}
          </div>
        </div>

        <div className="flex flex-wrap gap-4 text-sm shrink-0">
          <div className="text-center">
            <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">Duration</div>
            <div className="text-slate-200 tabular-nums font-semibold" style={{ fontFamily: 'JetBrains Mono' }}>
              {trace.duration_ms != null ? `${trace.duration_ms}ms` : '—'}
            </div>
          </div>
          <div className="text-center">
            <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">Cost</div>
            <div className="text-emerald-400 tabular-nums font-semibold" style={{ fontFamily: 'JetBrains Mono' }}>
              {trace.total_cost_usd != null ? `$${trace.total_cost_usd.toFixed(4)}` : '—'}
            </div>
          </div>
          <div className="text-center">
            <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">Spans</div>
            <div className="text-slate-200 tabular-nums font-semibold" style={{ fontFamily: 'JetBrains Mono' }}>
              {trace.span_count ?? trace.spans?.length ?? 0}
            </div>
          </div>
          {trace.peak_risk_score != null && (
            <div className="text-center">
              <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">Peak Risk</div>
              <div
                className="tabular-nums font-semibold"
                style={{
                  fontFamily: 'JetBrains Mono',
                  color: trace.peak_risk_score < 30 ? '#22c55e' : trace.peak_risk_score < 70 ? '#eab308' : '#ef4444',
                }}
              >
                {trace.peak_risk_score}/100
              </div>
            </div>
          )}
          {trace.started_at && (
            <div className="text-center">
              <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-1">Started</div>
              <div
                className="text-slate-400 text-xs"
                style={{ fontFamily: 'JetBrains Mono' }}
              >
                {format(parseISO(trace.started_at), 'HH:mm:ss')}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// ── Trace summary sidebar ─────────────────────────────────────────────────────

function TraceSummary({ spans }: { trace?: Trace; spans: SpanTreeNode[] }) {
  const llmSpans = spans.filter(s => s.event_kind.startsWith('llm'))
  const toolSpans = spans.filter(s => s.event_kind.startsWith('tool'))
  const errorSpans = spans.filter(s => s.event_kind === 'policy.block' || s.error_message)
  const totalTokens = spans.reduce((s, sp) => s + (sp.total_tokens ?? 0), 0)
  const totalCost = spans.reduce((s, sp) => s + (sp.cost_usd ?? 0), 0)

  const items = [
    { label: 'Total Spans', value: String(spans.length) },
    { label: 'LLM Calls', value: String(llmSpans.length) },
    { label: 'Tool Calls', value: String(toolSpans.length) },
    { label: 'Errors / Blocks', value: String(errorSpans.length), highlight: errorSpans.length > 0 },
    { label: 'Total Tokens', value: totalTokens.toLocaleString() },
    { label: 'Total Cost', value: `$${totalCost.toFixed(4)}`, green: true },
  ]

  return (
    <div className="glass-card p-5 space-y-4">
      <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider">Summary</h3>
      <div className="space-y-3">
        {items.map(item => (
          <div key={item.label} className="flex items-center justify-between">
            <span className="text-xs text-slate-500">{item.label}</span>
            <span
              className={clsx(
                'text-xs font-semibold tabular-nums',
                item.highlight ? 'text-red-400' :
                item.green ? 'text-emerald-400' :
                'text-slate-200',
              )}
              style={{ fontFamily: 'JetBrains Mono' }}
            >
              {item.value}
            </span>
          </div>
        ))}
      </div>

      {/* Event kind legend */}
      <div className="pt-3 border-t border-slate-700/50 space-y-2">
        <div className="text-[10px] text-slate-600 uppercase tracking-wider">Kind legend</div>
        {Object.entries(KIND_CONFIG).slice(0, 6).map(([kind, cfg]) => {
          const Icon = cfg.icon
          const count = spans.filter(s => s.event_kind === kind).length
          if (count === 0) return null
          return (
            <div key={kind} className="flex items-center gap-2">
              <Icon className="w-3 h-3 shrink-0" style={{ color: cfg.color }} />
              <span className="text-[11px] text-slate-500 flex-1 truncate">{cfg.label}</span>
              <span
                className="text-[11px] tabular-nums text-slate-400"
                style={{ fontFamily: 'JetBrains Mono' }}
              >
                {count}
              </span>
            </div>
          )
        })}
      </div>
    </div>
  )
}

// ── Waterfall ─────────────────────────────────────────────────────────────────

function TraceWaterfall({ trace }: { trace: Trace }) {
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set())

  const flatSpans = useMemo(() => {
    if (!trace.spans?.length) return []
    const tree = buildSpanTree(trace.spans)
    return flattenTree(tree)
  }, [trace.spans])

  const maxLatency = useMemo(
    () => Math.max(...flatSpans.map(s => s.latency_ms ?? 0), 1),
    [flatSpans],
  )

  const traceStart = useMemo(
    () => Math.min(...flatSpans.map(s => new Date(s.started_at).getTime())),
    [flatSpans],
  )

  const traceEnd = useMemo(() => {
    const ends = flatSpans.map(s =>
      s.ended_at
        ? new Date(s.ended_at).getTime()
        : new Date(s.started_at).getTime() + (s.latency_ms ?? 0)
    )
    return Math.max(...ends, traceStart + 1)
  }, [flatSpans, traceStart])

  function toggle(id: string) {
    setExpandedIds(prev => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  if (!flatSpans.length) {
    return (
      <div className="glass-card p-10 text-center">
        <div className="text-slate-500 text-sm">No spans found for this trace.</div>
        <div className="text-slate-600 text-xs mt-2">
          Spans appear after agents make API calls through the proxy.
        </div>
      </div>
    )
  }

  return (
    <div className="glass-card overflow-hidden">
      {/* Table header */}
      <div
        className="flex items-center gap-3 px-4 py-2.5"
        style={{
          background: 'rgba(255,255,255,0.015)',
          borderBottom: '1px solid rgba(148,163,184,0.07)',
        }}
      >
        <div className="w-3.5 shrink-0" />
        <div className="w-3.5 shrink-0" />
        <div className="w-40 shrink-0 text-[10px] font-semibold text-slate-600 uppercase tracking-wider">
          Span
        </div>
        <div className="flex-1 text-[10px] font-semibold text-slate-600 uppercase tracking-wider">
          Timeline
        </div>
        <div className="w-24 shrink-0 text-[10px] font-semibold text-slate-600 uppercase tracking-wider">
          Relative
        </div>
        <div className="w-16 shrink-0 text-right text-[10px] font-semibold text-slate-600 uppercase tracking-wider">
          Latency
        </div>
        <div className="w-20 shrink-0 text-[10px] font-semibold text-slate-600 uppercase tracking-wider">
          Risk
        </div>
        <div className="w-20 shrink-0 text-right text-[10px] font-semibold text-slate-600 uppercase tracking-wider">
          Time
        </div>
      </div>

      {/* Rows */}
      <div>
        {flatSpans.map(span => (
          <SpanRow
            key={span.id}
            span={span}
            maxLatency={maxLatency}
            traceStart={traceStart}
            traceEnd={traceEnd}
            isExpanded={expandedIds.has(span.id)}
            onToggle={() => toggle(span.id)}
          />
        ))}
      </div>
    </div>
  )
}

// ── TracePage ─────────────────────────────────────────────────────────────────

export function TracePage() {
  const { traceId } = useParams<{ traceId: string }>()

  const { data: trace, isLoading } = useQuery({
    queryKey: ['traces', traceId],
    queryFn: () => fetchTrace(traceId!),
    staleTime: 10_000,
    enabled: !!traceId,
  })

  const displayTrace = trace ?? null

  const flatSpans = useMemo(() => {
    if (!displayTrace?.spans?.length) return []
    const tree = buildSpanTree(displayTrace.spans)
    return flattenTree(tree)
  }, [displayTrace])

  return (
    <div className="space-y-4 stagger-in">
      {/* Back link */}
      <div>
        <Link
          to="/traces"
          className="inline-flex items-center gap-1.5 text-xs text-slate-500 hover:text-brand-400 transition-colors"
        >
          <ArrowLeft className="w-3.5 h-3.5" />
          All Traces
        </Link>
      </div>

      {isLoading ? (
        <div className="space-y-4">
          <div className="glass-card p-5 animate-pulse">
            <div className="h-6 bg-slate-700/50 rounded w-64 mb-3" />
            <div className="h-4 bg-slate-700/30 rounded w-96" />
          </div>
        </div>
      ) : !displayTrace ? (
        <div className="glass-card p-10 text-center">
          <div className="text-slate-400 text-sm">Trace not found.</div>
          <Link to="/traces" className="text-brand-400 text-xs mt-2 inline-block">
            Back to traces
          </Link>
        </div>
      ) : (
        <>
          <TraceHeader trace={displayTrace} />

          <div className="flex gap-4">
            {/* Waterfall */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-3">
                <h2 className="text-sm font-semibold text-slate-200 font-display">Waterfall</h2>
                <span className="text-xs text-slate-500">{flatSpans.length} spans · click to expand</span>
              </div>
              <TraceWaterfall trace={displayTrace} />
            </div>

            {/* Sidebar */}
            <div className="w-56 shrink-0">
              <div className="mb-3 text-sm font-semibold text-slate-200 font-display">Summary</div>
              <TraceSummary trace={displayTrace} spans={flatSpans} />
            </div>
          </div>
        </>
      )}
    </div>
  )
}

// Re-export StatusBadge usage
export { StatusBadge }
