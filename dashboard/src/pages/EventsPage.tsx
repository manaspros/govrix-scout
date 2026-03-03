import { useState } from 'react'
import { useEvents } from '../api/hooks'
import StatusBadge from '../components/common/StatusBadge'
import LoadingState from '../components/common/LoadingState'
import EmptyState from '../components/common/EmptyState'
import type { AgentEvent } from '../api/types'

const PAGE_SIZE = 25

function EventRow({ event }: { event: AgentEvent }) {
  const [expanded, setExpanded] = useState(false)
  return (
    <>
      <tr
        className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50 cursor-pointer"
        onClick={() => setExpanded(e => !e)}
      >
        <td className="py-2.5 px-3 font-mono text-xs text-slate-500">{event.id.slice(0, 8)}</td>
        <td className="py-2.5 px-3 text-xs text-slate-600 dark:text-slate-400">{event.kind}</td>
        <td className="py-2.5 px-3 text-xs text-slate-500 font-mono truncate max-w-[120px]">{event.agent_id.slice(0, 12)}</td>
        <td className="py-2.5 px-3 text-xs">{event.model ?? '—'}</td>
        <td className="py-2.5 px-3 text-xs">{event.cost_usd != null ? `$${event.cost_usd.toFixed(5)}` : '—'}</td>
        <td className="py-2.5 px-3"><StatusBadge status={event.pii_detected ? 'blocked' : 'ok'} /></td>
        <td className="py-2.5 px-3 text-xs text-slate-400">{new Date(event.timestamp).toLocaleString()}</td>
      </tr>
      {expanded && (
        <tr className="bg-slate-50 dark:bg-[#0d0d1a]">
          <td colSpan={7} className="px-4 py-3">
            <div className="grid grid-cols-2 gap-4 text-xs">
              <div>
                <p className="text-slate-400 mb-1">Session ID</p>
                <p className="font-mono text-slate-700 dark:text-slate-300 break-all">{event.session_id}</p>
              </div>
              <div>
                <p className="text-slate-400 mb-1">Lineage Hash</p>
                <p className="font-mono text-slate-700 dark:text-slate-300 break-all">{event.lineage_hash}</p>
              </div>
              <div>
                <p className="text-slate-400 mb-1">Compliance Tag</p>
                <p className="font-mono text-slate-700 dark:text-slate-300">{event.compliance_tag}</p>
              </div>
              <div>
                <p className="text-slate-400 mb-1">Latency</p>
                <p className="text-slate-700 dark:text-slate-300">{event.latency_ms != null ? `${event.latency_ms}ms` : '—'}</p>
              </div>
            </div>
          </td>
        </tr>
      )}
    </>
  )
}

export default function EventsPage() {
  const [page, setPage] = useState(0)
  const { data, isLoading } = useEvents({ limit: PAGE_SIZE, offset: page * PAGE_SIZE })

  if (isLoading) return <LoadingState />

  const total = data?.total ?? 0
  const pages = Math.ceil(total / PAGE_SIZE)

  return (
    <div className="card overflow-hidden">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300">
          Event Stream <span className="text-slate-400 font-normal ml-1">({total.toLocaleString()} total)</span>
        </h2>
      </div>
      {!data?.data.length ? <EmptyState message="No events yet" /> : (
        <>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                  {['ID', 'Kind', 'Agent', 'Model', 'Cost', 'PII', 'Time'].map(h => (
                    <th key={h} className="pb-2 px-3 font-medium">{h}</th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-50 dark:divide-slate-800/30">
                {data.data.map(e => <EventRow key={e.id} event={e} />)}
              </tbody>
            </table>
          </div>
          {pages > 1 && (
            <div className="flex items-center justify-between mt-4 pt-4 border-t border-slate-100 dark:border-slate-800">
              <button onClick={() => setPage(p => Math.max(0, p - 1))} disabled={page === 0} className="text-sm text-slate-500 disabled:opacity-40 hover:text-slate-700">← Prev</button>
              <span className="text-xs text-slate-400">{page + 1} / {pages}</span>
              <button onClick={() => setPage(p => Math.min(pages - 1, p + 1))} disabled={page >= pages - 1} className="text-sm text-slate-500 disabled:opacity-40 hover:text-slate-700">Next →</button>
            </div>
          )}
        </>
      )}
    </div>
  )
}
