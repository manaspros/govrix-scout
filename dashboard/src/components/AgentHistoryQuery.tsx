import { useState } from 'react'
import { Link } from 'react-router-dom'
import { format, parseISO } from 'date-fns'
import { Search } from 'lucide-react'
import { useQuery } from '@tanstack/react-query'
import { fetchEvents } from '@/api/client'
import type { AgentEvent } from '@/api/types'

// ── Result row ────────────────────────────────────────────────────────────────

function resultSummary(obj: Record<string, unknown> | undefined): string {
  if (!obj) return '—'
  const keys = Object.keys(obj)
  if (keys.length === 0) return '{}'
  const first = keys[0]
  const val = obj[first]
  const str = typeof val === 'string' ? val : JSON.stringify(val)
  return `${first}: ${str.slice(0, 60)}${str.length > 60 ? '…' : ''}`
}

function HistoryRow({ event }: { event: AgentEvent }) {
  const tool = (event.payload as Record<string, unknown>)?.tool_name as string | undefined
  const args = (event.payload as Record<string, unknown>)?.tool_args as Record<string, unknown> | undefined
  const result = (event.payload as Record<string, unknown>)?.tool_result as Record<string, unknown> | undefined

  return (
    <tr className="hover:bg-slate-700/20 transition-colors text-xs border-b border-slate-700/40">
      <td className="px-4 py-2.5 text-slate-500 font-mono whitespace-nowrap">
        {format(parseISO(event.timestamp), 'MMM d HH:mm:ss')}
      </td>
      <td className="px-4 py-2.5 text-slate-300 font-mono">
        {tool ?? event.model ?? '—'}
      </td>
      <td className="px-4 py-2.5 text-slate-500 max-w-[200px] truncate">
        {resultSummary(args)}
      </td>
      <td className="px-4 py-2.5 text-slate-500 max-w-[200px] truncate">
        {resultSummary(result)}
      </td>
      <td className="px-4 py-2.5">
        {event.session_id ? (
          <Link
            to={`/traces/${event.session_id}`}
            className="text-brand-400 hover:text-brand-300 font-mono text-[10px] transition-colors"
            onClick={e => e.stopPropagation()}
          >
            {event.session_id.slice(0, 12)}…
          </Link>
        ) : (
          <span className="text-slate-600">—</span>
        )}
      </td>
    </tr>
  )
}

// ── AgentHistoryQuery ─────────────────────────────────────────────────────────

interface AgentHistoryQueryProps {
  agentId: string
}

export function AgentHistoryQuery({ agentId }: AgentHistoryQueryProps) {
  const [toolName, setToolName] = useState('')
  const [fromDate, setFromDate] = useState('')
  const [toDate, setToDate] = useState('')
  const [submitted, setSubmitted] = useState(false)

  const { data, isLoading } = useQuery({
    queryKey: ['agent-history', agentId, toolName, fromDate, toDate, submitted],
    queryFn: () =>
      fetchEvents({
        agent_id: agentId,
        kind: 'tool.invoke',
        since: fromDate || undefined,
        until: toDate || undefined,
        limit: 50,
      }),
    enabled: submitted,
    staleTime: 30_000,
  })

  const events = data?.events ?? []

  return (
    <div className="space-y-4">
      {/* Query panel */}
      <div className="glass-card p-5">
        <div className="flex items-center gap-2 mb-4">
          <Search className="w-4 h-4 text-brand-400" />
          <span className="text-sm font-semibold text-slate-200 font-display">Search agent history</span>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-3">
          {/* Agent (read-only) */}
          <div>
            <label className="block text-[10px] text-slate-500 uppercase tracking-wider mb-1.5">
              Agent
            </label>
            <div className="px-3 py-2 text-sm bg-slate-800/80 border border-slate-600 rounded-lg text-slate-400 font-mono truncate">
              {agentId}
            </div>
          </div>

          {/* Tool */}
          <div>
            <label className="block text-[10px] text-slate-500 uppercase tracking-wider mb-1.5">
              Tool (optional)
            </label>
            <input
              type="text"
              placeholder="e.g. read_file"
              value={toolName}
              onChange={e => setToolName(e.target.value)}
              className="w-full px-3 py-2 text-sm bg-slate-800 border border-slate-600 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
            />
          </div>

          {/* From */}
          <div>
            <label className="block text-[10px] text-slate-500 uppercase tracking-wider mb-1.5">
              From
            </label>
            <input
              type="datetime-local"
              value={fromDate}
              onChange={e => setFromDate(e.target.value)}
              className="w-full px-3 py-2 text-sm bg-slate-800 border border-slate-600 rounded-lg text-slate-200 focus:outline-none focus:ring-1 focus:ring-brand-500"
            />
          </div>

          {/* To */}
          <div>
            <label className="block text-[10px] text-slate-500 uppercase tracking-wider mb-1.5">
              To
            </label>
            <input
              type="datetime-local"
              value={toDate}
              onChange={e => setToDate(e.target.value)}
              className="w-full px-3 py-2 text-sm bg-slate-800 border border-slate-600 rounded-lg text-slate-200 focus:outline-none focus:ring-1 focus:ring-brand-500"
            />
          </div>
        </div>

        <button
          onClick={() => setSubmitted(true)}
          className="mt-4 flex items-center gap-2 px-4 py-2 text-sm font-medium bg-brand-600 text-white rounded-lg hover:bg-brand-500 transition-colors"
        >
          <Search className="w-3.5 h-3.5" />
          Search
        </button>
      </div>

      {/* Results */}
      {submitted && (
        <div className="overflow-x-auto rounded-xl border border-slate-700">
          <table className="w-full text-sm">
            <thead className="bg-slate-800/80">
              <tr>
                {['Timestamp', 'Tool / Model', 'Args summary', 'Result summary', 'Trace'].map(h => (
                  <th
                    key={h}
                    className="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider whitespace-nowrap"
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {isLoading ? (
                Array.from({ length: 5 }).map((_, i) => (
                  <tr key={i} className="animate-pulse">
                    {Array.from({ length: 5 }).map((__, j) => (
                      <td key={j} className="px-4 py-3">
                        <div className="h-3 bg-slate-700/50 rounded" />
                      </td>
                    ))}
                  </tr>
                ))
              ) : events.length === 0 ? (
                <tr>
                  <td colSpan={5} className="px-4 py-8 text-center text-sm text-slate-500">
                    No tool events found for the given criteria.
                  </td>
                </tr>
              ) : (
                events.map(e => <HistoryRow key={e.id} event={e} />)
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
