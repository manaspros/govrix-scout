import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { format, parseISO } from 'date-fns'
import { GitBranch, Search, ChevronLeft, ChevronRight, RefreshCw } from 'lucide-react'
import { clsx } from 'clsx'
import { fetchTraces } from '@/api/traces'
import type { Trace, TraceStatus } from '@/api/traces'
import { EmptyState } from '@/components/common/EmptyState'

const PAGE_SIZE = 25

// ── Status badge ──────────────────────────────────────────────────────────────

const STATUS_STYLES: Record<TraceStatus, { dot?: string; cls: string; pulse?: boolean }> = {
  running:   { dot: '#eab308', cls: 'bg-yellow-500/20 text-yellow-400 ring-1 ring-yellow-500/40', pulse: true },
  completed: { dot: '#22c55e', cls: 'bg-emerald-500/20 text-emerald-400 ring-1 ring-emerald-500/40' },
  stopped:   { dot: '#6b7280', cls: 'bg-slate-500/20 text-slate-400 ring-1 ring-slate-500/40' },
  failed:    { dot: '#ef4444', cls: 'bg-red-500/20 text-red-400 ring-1 ring-red-500/40' },
}

function TraceStatusBadge({ status }: { status: TraceStatus }) {
  const s = STATUS_STYLES[status] ?? STATUS_STYLES.stopped
  return (
    <span className={clsx('inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium', s.cls)}>
      {s.dot && (
        <span
          className={clsx('w-1.5 h-1.5 rounded-full shrink-0', s.pulse && 'animate-pulse')}
          style={{ background: s.dot }}
        />
      )}
      {status}
    </span>
  )
}

// ── Risk indicator ────────────────────────────────────────────────────────────

function RiskPill({ score }: { score: number }) {
  const color = score < 30 ? '#22c55e' : score < 70 ? '#eab308' : '#ef4444'
  return (
    <span
      className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium tabular-nums"
      style={{
        color,
        background: `${color}18`,
        border: `1px solid ${color}33`,
        fontFamily: 'JetBrains Mono',
      }}
    >
      {score}
    </span>
  )
}

// ── TraceListPage ─────────────────────────────────────────────────────────────

const STATUS_OPTIONS: (TraceStatus | 'all')[] = ['all', 'running', 'completed', 'stopped', 'failed']

export function TraceListPage() {
  const navigate = useNavigate()
  const [agentFilter, setAgentFilter] = useState('')
  const [statusFilter, setStatusFilter] = useState<TraceStatus | 'all'>('all')
  const [page, setPage] = useState(0)

  const { data, isLoading, refetch, isFetching } = useQuery({
    queryKey: ['traces', { agentFilter, statusFilter, page }],
    queryFn: () =>
      fetchTraces({
        agent_id: agentFilter || undefined,
        status: statusFilter !== 'all' ? statusFilter : undefined,
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE,
      }),
    staleTime: 10_000,
  })

  const traces: Trace[] = data?.traces ?? []

  const total = data?.total ?? traces.length
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE))

  return (
    <div className="space-y-4 stagger-in">
      {/* Toolbar */}
      <div className="flex flex-wrap items-center gap-3">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
          <input
            type="text"
            placeholder="Filter by agent…"
            value={agentFilter}
            onChange={e => { setAgentFilter(e.target.value); setPage(0) }}
            className="pl-9 pr-3 py-2 text-sm bg-slate-800 border border-slate-600 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:ring-1 focus:ring-brand-500 w-48"
          />
        </div>

        <select
          value={statusFilter}
          onChange={e => { setStatusFilter(e.target.value as TraceStatus | 'all'); setPage(0) }}
          className="px-3 py-2 text-sm bg-slate-800 border border-slate-600 rounded-lg text-slate-200 focus:outline-none focus:ring-1 focus:ring-brand-500"
        >
          {STATUS_OPTIONS.map(s => (
            <option key={s} value={s}>{s === 'all' ? 'All statuses' : s}</option>
          ))}
        </select>

        <button
          onClick={() => void refetch()}
          disabled={isFetching}
          className="flex items-center gap-1.5 px-3 py-2 text-xs bg-slate-700 text-slate-300 rounded-lg hover:bg-slate-600 transition-colors disabled:opacity-50"
        >
          <RefreshCw className={clsx('w-3.5 h-3.5', isFetching && 'animate-spin')} />
          Refresh
        </button>

        <span className="text-xs text-slate-500 ml-auto">{total} traces</span>
      </div>

      {/* Table */}
      <div className="overflow-x-auto rounded-xl border border-slate-700">
        <table className="w-full text-sm">
          <thead className="bg-slate-800/80">
            <tr>
              {['Trace ID', 'Agent', 'Status', 'Started', 'Duration', 'Spans', 'Cost', 'Peak Risk'].map(h => (
                <th
                  key={h}
                  className="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider whitespace-nowrap"
                >
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-700/50">
            {isLoading ? (
              Array.from({ length: 8 }).map((_, i) => (
                <tr key={i} className="animate-pulse">
                  {Array.from({ length: 8 }).map((__, j) => (
                    <td key={j} className="px-4 py-3">
                      <div className="h-4 bg-slate-700/50 rounded" />
                    </td>
                  ))}
                </tr>
              ))
            ) : traces.length === 0 ? (
              <tr>
                <td colSpan={8}>
                  <EmptyState
                    icon={GitBranch}
                    title="No traces found"
                    description="Traces appear when agents make multi-step calls through the proxy."
                  />
                </td>
              </tr>
            ) : (
              traces.map(trace => (
                <tr
                  key={trace.trace_id}
                  className="cursor-pointer hover:bg-slate-700/25 transition-colors"
                  onClick={() => navigate(`/traces/${trace.trace_id}`)}
                >
                  {/* Trace ID */}
                  <td className="px-4 py-3">
                    <span
                      className="text-xs text-brand-400 font-mono hover:text-brand-300"
                      title={trace.trace_id}
                    >
                      {trace.trace_id.slice(0, 20)}…
                    </span>
                  </td>

                  {/* Agent */}
                  <td className="px-4 py-3 text-sm text-slate-200 font-medium max-w-[140px] truncate">
                    {trace.root_agent}
                  </td>

                  {/* Status */}
                  <td className="px-4 py-3">
                    <TraceStatusBadge status={trace.status} />
                  </td>

                  {/* Started */}
                  <td
                    className="px-4 py-3 text-xs text-slate-400 whitespace-nowrap"
                    style={{ fontFamily: 'JetBrains Mono' }}
                  >
                    {format(parseISO(trace.started_at), 'MMM d HH:mm:ss')}
                  </td>

                  {/* Duration */}
                  <td
                    className="px-4 py-3 text-xs text-slate-400 tabular-nums"
                    style={{ fontFamily: 'JetBrains Mono' }}
                  >
                    {trace.duration_ms != null ? `${trace.duration_ms}ms` : '—'}
                  </td>

                  {/* Spans */}
                  <td
                    className="px-4 py-3 text-xs text-slate-400 tabular-nums text-center"
                    style={{ fontFamily: 'JetBrains Mono' }}
                  >
                    {trace.span_count ?? '—'}
                  </td>

                  {/* Cost */}
                  <td
                    className="px-4 py-3 text-xs text-emerald-400 tabular-nums"
                    style={{ fontFamily: 'JetBrains Mono' }}
                  >
                    {trace.total_cost_usd != null ? `$${trace.total_cost_usd.toFixed(4)}` : '—'}
                  </td>

                  {/* Peak risk */}
                  <td className="px-4 py-3">
                    {trace.peak_risk_score != null
                      ? <RiskPill score={trace.peak_risk_score} />
                      : <span className="text-xs text-slate-600">—</span>
                    }
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between text-xs text-slate-500">
          <span>
            Page {page + 1} of {totalPages} · {total} traces
          </span>
          <div className="flex items-center gap-2">
            <button
              onClick={() => setPage(p => Math.max(0, p - 1))}
              disabled={page === 0}
              className="p-1.5 rounded bg-slate-800 border border-slate-700 hover:bg-slate-700 disabled:opacity-40 transition-colors"
            >
              <ChevronLeft className="w-3.5 h-3.5" />
            </button>
            <button
              onClick={() => setPage(p => Math.min(totalPages - 1, p + 1))}
              disabled={page >= totalPages - 1}
              className="p-1.5 rounded bg-slate-800 border border-slate-700 hover:bg-slate-700 disabled:opacity-40 transition-colors"
            >
              <ChevronRight className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
