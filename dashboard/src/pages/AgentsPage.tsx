import { useState } from 'react'
import { format, parseISO } from 'date-fns'
import { Search, ChevronDown, ChevronRight, Bot, RefreshCw } from 'lucide-react'
import { useAgents } from '@/api/hooks'
import { StatusBadge } from '@/components/common/StatusBadge'
import { EmptyState } from '@/components/common/EmptyState'
import type { Agent, AgentStatus } from '@/api/types'

// ── Agent detail expand panel ─────────────────────────────────────────────────

function AgentDetail({ agent }: { agent: Agent }) {
  return (
    <div className="bg-slate-900/60 px-6 py-4 border-t border-slate-700/40 grid grid-cols-2 md:grid-cols-4 gap-4 text-xs">
      <div>
        <div className="text-slate-500 uppercase tracking-wider mb-1">Agent ID</div>
        <div className="text-slate-300 font-mono break-all">{agent.id}</div>
      </div>
      <div>
        <div className="text-slate-500 uppercase tracking-wider mb-1">Description</div>
        <div className="text-slate-300">{agent.description ?? '—'}</div>
      </div>
      <div>
        <div className="text-slate-500 uppercase tracking-wider mb-1">First Seen</div>
        <div className="text-slate-300">
          {agent.first_seen_at ? format(parseISO(agent.first_seen_at), 'PPpp') : '—'}
        </div>
      </div>
      <div>
        <div className="text-slate-500 uppercase tracking-wider mb-1">Source IP</div>
        <div className="text-slate-300 font-mono">{agent.source_ip ?? '—'}</div>
      </div>
      <div>
        <div className="text-slate-500 uppercase tracking-wider mb-1">Labels</div>
        <div className="text-slate-300">
          {Object.keys(agent.labels ?? {}).length > 0
            ? Object.entries(agent.labels).map(([k, v]) => (
                <span key={k} className="inline-block mr-1 mb-1 px-1.5 py-0.5 bg-slate-700 rounded text-slate-300">
                  {k}={String(v)}
                </span>
              ))
            : '—'}
        </div>
      </div>
      <div>
        <div className="text-slate-500 uppercase tracking-wider mb-1">Last Error</div>
        <div className="text-slate-300">
          {agent.last_error_at ? format(parseISO(agent.last_error_at), 'PPpp') : '—'}
        </div>
      </div>
      <div>
        <div className="text-slate-500 uppercase tracking-wider mb-1">Error Count</div>
        <div className={agent.error_count > 0 ? 'text-red-400' : 'text-slate-300'}>
          {agent.error_count}
        </div>
      </div>
      <div>
        <div className="text-slate-500 uppercase tracking-wider mb-1">Total Cost</div>
        <div className="text-slate-300 tabular-nums">${(agent.total_cost_usd ?? 0).toFixed(6)}</div>
      </div>
    </div>
  )
}

// ── Agents row ────────────────────────────────────────────────────────────────

function AgentRow({ agent }: { agent: Agent }) {
  const [expanded, setExpanded] = useState(false)

  return (
    <>
      <tr
        className="cursor-pointer hover:bg-slate-700/20 transition-colors"
        onClick={() => setExpanded(e => !e)}
      >
        <td className="px-4 py-3">
          {expanded
            ? <ChevronDown className="w-4 h-4 text-slate-400" />
            : <ChevronRight className="w-4 h-4 text-slate-400" />}
        </td>
        <td className="px-4 py-3">
          <div className="flex flex-col">
            <span className="text-sm font-medium text-slate-200">{agent.name ?? 'Unnamed'}</span>
            <span className="text-xs text-slate-500 font-mono">{agent.id.slice(0, 16)}…</span>
          </div>
        </td>
        <td className="px-4 py-3 text-sm text-slate-400">{agent.framework ?? '—'}</td>
        <td className="px-4 py-3">
          <StatusBadge value={agent.status} />
        </td>
        <td className="px-4 py-3 text-sm text-slate-300 tabular-nums text-right">
          {(agent.total_requests ?? 0).toLocaleString()}
        </td>
        <td className="px-4 py-3 text-sm text-slate-300 tabular-nums text-right">
          {(agent.total_tokens ?? 0).toLocaleString()}
        </td>
        <td className="px-4 py-3 text-sm text-slate-300 tabular-nums text-right">
          ${(agent.total_cost_usd ?? 0).toFixed(4)}
        </td>
        <td className="px-4 py-3 text-xs text-slate-500">
          {agent.last_model_used ?? '—'}
        </td>
        <td className="px-4 py-3 text-xs text-slate-500 whitespace-nowrap">
          {agent.last_seen_at
            ? format(parseISO(agent.last_seen_at), 'MMM d, HH:mm')
            : '—'}
        </td>
      </tr>
      {expanded && (
        <tr>
          <td colSpan={9} className="p-0">
            <AgentDetail agent={agent} />
          </td>
        </tr>
      )}
    </>
  )
}

// ── Agents page ───────────────────────────────────────────────────────────────

const STATUS_OPTIONS: (AgentStatus | 'all')[] = ['all', 'active', 'idle', 'error', 'blocked', 'retired']

export function AgentsPage() {
  const [search, setSearch] = useState('')
  const [statusFilter, setStatusFilter] = useState<AgentStatus | 'all'>('all')

  const { data, isLoading, refetch, isFetching } = useAgents()

  const agents = data?.agents ?? []
  const filtered = agents.filter(a => {
    const matchSearch = !search ||
      (a.name ?? '').toLowerCase().includes(search.toLowerCase()) ||
      a.id.toLowerCase().includes(search.toLowerCase())
    const matchStatus = statusFilter === 'all' || a.status === statusFilter
    return matchSearch && matchStatus
  })

  return (
    <div className="space-y-4">
      {/* Toolbar */}
      <div className="flex flex-col sm:flex-row gap-3 items-start sm:items-center justify-between">
        <div className="flex gap-3 flex-1 w-full sm:w-auto">
          {/* Search */}
          <div className="relative flex-1 max-w-xs">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
            <input
              type="text"
              placeholder="Search agents…"
              value={search}
              onChange={e => setSearch(e.target.value)}
              className="w-full pl-9 pr-3 py-2 text-sm bg-slate-800 border border-slate-600 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
            />
          </div>
          {/* Status filter */}
          <select
            value={statusFilter}
            onChange={e => setStatusFilter(e.target.value as AgentStatus | 'all')}
            className="px-3 py-2 text-sm bg-slate-800 border border-slate-600 rounded-lg text-slate-200 focus:outline-none focus:ring-1 focus:ring-brand-500"
          >
            {STATUS_OPTIONS.map(s => (
              <option key={s} value={s}>{s === 'all' ? 'All statuses' : s}</option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-3">
          <span className="text-xs text-slate-500">{filtered.length} agents</span>
          <button
            onClick={() => void refetch()}
            disabled={isFetching}
            className="flex items-center gap-1.5 px-3 py-2 text-xs bg-slate-700 text-slate-300 rounded-lg hover:bg-slate-600 transition-colors disabled:opacity-50"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isFetching ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>
      </div>

      {/* Table */}
      <div className="overflow-x-auto rounded-xl border border-slate-700">
        <table className="w-full text-sm">
          <thead className="bg-slate-800/80">
            <tr>
              <th className="px-4 py-3 w-8" />
              {['Name / ID', 'Framework', 'Status', 'Requests', 'Tokens', 'Cost', 'Last Model', 'Last Seen'].map(h => (
                <th key={h} className="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider whitespace-nowrap">
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-700/50">
            {isLoading ? (
              Array.from({ length: 6 }).map((_, i) => (
                <tr key={i} className="animate-pulse">
                  {Array.from({ length: 9 }).map((__, j) => (
                    <td key={j} className="px-4 py-3">
                      <div className="h-4 bg-slate-700/50 rounded" />
                    </td>
                  ))}
                </tr>
              ))
            ) : filtered.length === 0 ? (
              <tr>
                <td colSpan={9}>
                  <EmptyState
                    icon={Bot}
                    title="No agents found"
                    description="Connect your AI agents by setting SCOUT_PROXY_URL"
                  />
                </td>
              </tr>
            ) : (
              filtered.map(agent => <AgentRow key={agent.id} agent={agent} />)
            )}
          </tbody>
        </table>
      </div>
    </div>
  )
}
