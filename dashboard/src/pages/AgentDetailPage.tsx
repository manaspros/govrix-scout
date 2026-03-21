import { useState } from 'react'
import { useParams, useNavigate, Link } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { format, parseISO } from 'date-fns'
import { ArrowLeft, Bot, GitBranch, History, RefreshCw, ChevronRight } from 'lucide-react'
import { clsx } from 'clsx'
import { fetchAgent } from '@/api/client'
import { fetchTraces } from '@/api/traces'
import type { Trace, TraceStatus } from '@/api/traces'
import { StatusBadge } from '@/components/common/StatusBadge'
import { AgentHistoryQuery } from '@/components/AgentHistoryQuery'

// ── Tab types ─────────────────────────────────────────────────────────────────

type TabId = 'overview' | 'traces' | 'history'

const TABS: { id: TabId; label: string; icon: React.ComponentType<{ className?: string }> }[] = [
  { id: 'overview', label: 'Overview',   icon: Bot },
  { id: 'traces',   label: 'Traces',     icon: GitBranch },
  { id: 'history',  label: 'History',    icon: History },
]

// ── Trace status badge ────────────────────────────────────────────────────────

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

// ── Overview tab ──────────────────────────────────────────────────────────────

function OverviewTab({ agentId }: { agentId: string }) {
  const { data: agent, isLoading } = useQuery({
    queryKey: ['agents', agentId],
    queryFn: () => fetchAgent(agentId),
    staleTime: 15_000,
  })

  if (isLoading) {
    return (
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4 animate-pulse">
        {Array.from({ length: 8 }).map((_, i) => (
          <div key={i} className="glass-card p-4">
            <div className="h-3 bg-slate-700/50 rounded w-20 mb-2" />
            <div className="h-5 bg-slate-700/30 rounded w-28" />
          </div>
        ))}
      </div>
    )
  }

  if (!agent) {
    return <div className="text-sm text-slate-500">Agent not found.</div>
  }

  const fields: { label: string; value: React.ReactNode }[] = [
    { label: 'Agent ID',    value: <span className="font-mono text-slate-300 break-all">{agent.id}</span> },
    { label: 'Status',      value: <StatusBadge value={agent.status} /> },
    { label: 'Framework',   value: agent.framework ?? '—' },
    { label: 'Description', value: agent.description ?? '—' },
    { label: 'Requests',    value: <span className="tabular-nums" style={{ fontFamily: 'JetBrains Mono' }}>{(agent.total_requests ?? 0).toLocaleString()}</span> },
    { label: 'Total Tokens',value: <span className="tabular-nums" style={{ fontFamily: 'JetBrains Mono' }}>{(agent.total_tokens ?? 0).toLocaleString()}</span> },
    { label: 'Total Cost',  value: <span className="text-emerald-400 tabular-nums" style={{ fontFamily: 'JetBrains Mono' }}>${(agent.total_cost_usd ?? 0).toFixed(4)}</span> },
    { label: 'Last Model',  value: agent.last_model_used ?? '—' },
    { label: 'First Seen',  value: agent.first_seen_at ? format(parseISO(agent.first_seen_at), 'PPp') : '—' },
    { label: 'Last Seen',   value: agent.last_seen_at ? format(parseISO(agent.last_seen_at), 'PPp') : '—' },
    { label: 'Error Count', value: <span className={agent.error_count > 0 ? 'text-red-400' : 'text-slate-300'}>{agent.error_count}</span> },
    { label: 'Source IP',   value: <span className="font-mono">{agent.source_ip ?? '—'}</span> },
  ]

  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-3">
      {fields.map(f => (
        <div key={String(f.label)} className="glass-card p-4">
          <div className="text-[10px] text-slate-500 uppercase tracking-wider mb-1.5">{f.label}</div>
          <div className="text-sm text-slate-200">{f.value}</div>
        </div>
      ))}
    </div>
  )
}

// ── Traces tab ────────────────────────────────────────────────────────────────

function TracesTab({ agentId }: { agentId: string }) {
  const { data, isLoading, refetch, isFetching } = useQuery({
    queryKey: ['traces', { agentId }],
    queryFn: () => fetchTraces({ agent_id: agentId, limit: 10 }),
    staleTime: 10_000,
  })

  const traces: Trace[] = data?.traces ?? []

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-xs text-slate-500">{data?.total ?? traces.length} total traces</span>
        <button
          onClick={() => void refetch()}
          disabled={isFetching}
          className="flex items-center gap-1.5 px-3 py-1.5 text-xs bg-slate-700 text-slate-300 rounded-lg hover:bg-slate-600 transition-colors disabled:opacity-50"
        >
          <RefreshCw className={clsx('w-3 h-3', isFetching && 'animate-spin')} />
          Refresh
        </button>
      </div>

      <div className="overflow-x-auto rounded-xl border border-slate-700">
        <table className="w-full text-sm">
          <thead className="bg-slate-800/80">
            <tr>
              {['Status', 'Started', 'Duration', 'Spans', 'Cost'].map(h => (
                <th
                  key={h}
                  className="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider"
                >
                  {h}
                </th>
              ))}
              <th className="px-4 py-3 w-8" />
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-700/40">
            {isLoading ? (
              Array.from({ length: 5 }).map((_, i) => (
                <tr key={i} className="animate-pulse">
                  {Array.from({ length: 6 }).map((__, j) => (
                    <td key={j} className="px-4 py-3">
                      <div className="h-4 bg-slate-700/50 rounded" />
                    </td>
                  ))}
                </tr>
              ))
            ) : traces.length === 0 ? (
              <tr>
                <td colSpan={6} className="px-4 py-8 text-center text-sm text-slate-500">
                  No traces found for this agent.
                </td>
              </tr>
            ) : (
              traces.map(trace => (
                <tr
                  key={trace.trace_id}
                  className="hover:bg-slate-700/20 transition-colors cursor-pointer"
                >
                  <td className="px-4 py-2.5">
                    <TraceStatusBadge status={trace.status} />
                  </td>
                  <td className="px-4 py-2.5 text-xs text-slate-400 font-mono whitespace-nowrap">
                    {format(parseISO(trace.started_at), 'MMM d HH:mm:ss')}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-slate-400 tabular-nums font-mono">
                    {trace.duration_ms != null ? `${trace.duration_ms}ms` : '—'}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-slate-400 tabular-nums text-center font-mono">
                    {trace.span_count ?? '—'}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-emerald-400 tabular-nums font-mono">
                    {trace.total_cost_usd != null ? `$${trace.total_cost_usd.toFixed(4)}` : '—'}
                  </td>
                  <td className="px-4 py-2.5">
                    <Link
                      to={`/traces/${trace.trace_id}`}
                      className="flex items-center gap-1 text-xs text-brand-400 hover:text-brand-300 transition-colors"
                    >
                      View <ChevronRight className="w-3.5 h-3.5" />
                    </Link>
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {traces.length > 0 && (
        <div className="text-right">
          <Link
            to={`/traces?agent_id=${agentId}`}
            className="text-xs text-brand-400 hover:text-brand-300 transition-colors"
          >
            View all traces for this agent →
          </Link>
        </div>
      )}
    </div>
  )
}

// ── AgentDetailPage ───────────────────────────────────────────────────────────

export function AgentDetailPage() {
  const { agentId } = useParams<{ agentId: string }>()
  const navigate = useNavigate()
  const [activeTab, setActiveTab] = useState<TabId>('overview')

  const { data: agent } = useQuery({
    queryKey: ['agents', agentId],
    queryFn: () => fetchAgent(agentId!),
    staleTime: 15_000,
    enabled: !!agentId,
  })

  if (!agentId) {
    return <div className="text-sm text-slate-500">No agent ID specified.</div>
  }

  return (
    <div className="space-y-4 stagger-in">
      {/* Back */}
      <div>
        <button
          onClick={() => navigate('/agents')}
          className="inline-flex items-center gap-1.5 text-xs text-slate-500 hover:text-brand-400 transition-colors"
        >
          <ArrowLeft className="w-3.5 h-3.5" />
          All Agents
        </button>
      </div>

      {/* Agent header */}
      <div className="glass-card p-5">
        <div className="flex items-center gap-4">
          <div
            className="flex items-center justify-center w-12 h-12 rounded-xl shrink-0"
            style={{ background: 'rgba(16,185,129,0.1)', border: '1px solid rgba(16,185,129,0.2)' }}
          >
            <Bot className="w-6 h-6 text-brand-400" />
          </div>
          <div>
            <div className="text-lg font-semibold text-slate-100 font-display">
              {agent?.name ?? agentId}
            </div>
            <div className="text-xs text-slate-500 font-mono mt-0.5">{agentId}</div>
          </div>
          {agent && (
            <div className="ml-auto">
              <StatusBadge value={agent.status} size="md" />
            </div>
          )}
        </div>
      </div>

      {/* Tabs */}
      <div
        className="flex items-center gap-1 p-1 rounded-xl"
        style={{ background: 'rgba(255,255,255,0.03)', border: '1px solid rgba(148,163,184,0.08)' }}
      >
        {TABS.map(tab => {
          const Icon = tab.icon
          const isActive = activeTab === tab.id
          return (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={clsx(
                'flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all duration-200',
                isActive
                  ? 'text-slate-100 shadow-sm'
                  : 'text-slate-500 hover:text-slate-300',
              )}
              style={isActive ? {
                background: 'rgba(16,185,129,0.12)',
                border: '1px solid rgba(16,185,129,0.2)',
              } : {}}
            >
              <Icon className={clsx('w-4 h-4', isActive ? 'text-brand-400' : 'text-slate-600')} />
              {tab.label}
            </button>
          )
        })}
      </div>

      {/* Tab content */}
      <div>
        {activeTab === 'overview' && <OverviewTab agentId={agentId} />}
        {activeTab === 'traces'   && <TracesTab agentId={agentId} />}
        {activeTab === 'history'  && <AgentHistoryQuery agentId={agentId} />}
      </div>
    </div>
  )
}
