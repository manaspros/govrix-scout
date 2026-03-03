import React, { useState } from 'react'
import { Radio, ChevronDown, ChevronUp, Filter, RefreshCw } from 'lucide-react'
import { useEvents } from '../api/hooks'
import type { AgentEvent } from '../api/types'

const PAGE_SIZE = 25

const fmtTime = (ts: string | undefined): string => {
  if (!ts) return '—'
  const d = new Date(ts)
  return d.toLocaleString([], {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

const statusBadge = (code: number | null | undefined): string => {
  if (code == null) return 'badge badge-neutral'
  if (code >= 400) return 'badge badge-danger'
  if (code >= 300) return 'badge badge-warning'
  return 'badge badge-success'
}

interface ExpandedRowProps {
  ev: AgentEvent
}

function ExpandedRow({ ev }: ExpandedRowProps) {
  return (
    <tr className="bg-slate-50/70">
      <td colSpan={9} className="px-6 py-4">
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-xs">
          <div>
            <span className="text-slate-400 block mb-1">Session ID</span>
            <span className="metric-font text-slate-600 text-[11px]">{ev.session_id || '—'}</span>
          </div>
          <div>
            <span className="text-slate-400 block mb-1">Protocol</span>
            <span className="text-slate-600">{ev.protocol || '—'}</span>
          </div>
          <div>
            <span className="text-slate-400 block mb-1">Input / Output Tokens</span>
            <span className="metric-font text-slate-600">
              {ev.input_tokens ?? '—'} / {ev.output_tokens ?? '—'}
            </span>
          </div>
          <div>
            <span className="text-slate-400 block mb-1">Lineage Hash</span>
            <span className="metric-font text-slate-600 text-[11px]">
              {ev.lineage_hash ? `${ev.lineage_hash.slice(0, 16)}...` : '—'}
            </span>
          </div>
          <div>
            <span className="text-slate-400 block mb-1">Compliance Tag</span>
            <span className="badge badge-info">{ev.compliance_tag || '—'}</span>
          </div>
          <div>
            <span className="text-slate-400 block mb-1">Latency</span>
            <span className="text-slate-600">
              {ev.latency_ms != null ? `${ev.latency_ms}ms` : '—'}
            </span>
          </div>
          <div>
            <span className="text-slate-400 block mb-1">PII Detected</span>
            <span className={`badge ${ev.pii_detected ? 'badge-warning' : 'badge-neutral'}`}>
              {ev.pii_detected ? 'Yes' : 'No'}
            </span>
          </div>
          <div>
            <span className="text-slate-400 block mb-1">Kind</span>
            <span className="text-slate-600 text-[11px]">{ev.kind || '—'}</span>
          </div>
        </div>
      </td>
    </tr>
  )
}

export default function EventsPage() {
  const [page, setPage] = useState(0)
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const [modelFilter, setModelFilter] = useState('')
  const [agentFilter, setAgentFilter] = useState('')

  const { data, isLoading, refetch } = useEvents({
    limit: PAGE_SIZE,
    offset: page * PAGE_SIZE,
    agent_id: agentFilter || undefined,
  })

  const allEvents = data?.data ?? []

  // Client-side model filter (useEvents doesn't support model param in types)
  const events = modelFilter
    ? allEvents.filter(e => (e.model ?? '').toLowerCase().includes(modelFilter.toLowerCase()))
    : allEvents

  const toggle = (id: string) => setExpandedId(prev => (prev === id ? null : id))

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-[1400px] mx-auto space-y-4">

        {/* Header */}
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-bold text-slate-900">Events</h2>
            <p className="text-xs text-slate-400">Real-time proxy event stream</p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={() => refetch()}
              className="btn-secondary flex items-center gap-1.5 text-xs"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />
              Refresh
            </button>
          </div>
        </div>

        {/* Filters */}
        <div className="flex items-center gap-3 flex-wrap">
          <div className="flex items-center gap-1.5 text-xs text-slate-500">
            <Filter className="w-3.5 h-3.5" /> Filters:
          </div>
          <input
            className="input-field w-40 text-xs py-1.5"
            placeholder="Agent ID..."
            value={agentFilter}
            onChange={e => { setAgentFilter(e.target.value); setPage(0) }}
          />
          <input
            className="input-field w-40 text-xs py-1.5"
            placeholder="Model..."
            value={modelFilter}
            onChange={e => { setModelFilter(e.target.value); setPage(0) }}
          />
          {(modelFilter || agentFilter) && (
            <button
              className="text-xs text-primary font-medium"
              onClick={() => { setModelFilter(''); setAgentFilter(''); setPage(0) }}
            >
              Clear
            </button>
          )}
        </div>

        {/* Table */}
        <div className="bg-white border border-slate-200 rounded-xl overflow-hidden">
          <table className="w-full">
            <thead>
              <tr className="border-b border-slate-100 bg-slate-50/50">
                <th className="table-header text-left py-3 px-4 w-8"></th>
                <th className="table-header text-left py-3 px-4">Time</th>
                <th className="table-header text-left py-3 px-4">Agent</th>
                <th className="table-header text-left py-3 px-4">Model</th>
                <th className="table-header text-left py-3 px-4">Provider</th>
                <th className="table-header text-right py-3 px-4">Tokens</th>
                <th className="table-header text-right py-3 px-4">Cost</th>
                <th className="table-header text-right py-3 px-4">Latency</th>
                <th className="table-header text-center py-3 px-4">Status</th>
              </tr>
            </thead>
            <tbody>
              {events.map((ev, i) => (
                <React.Fragment key={ev.id || i}>
                  <tr
                    className="border-b border-slate-50 hover:bg-slate-50/50 cursor-pointer transition-colors"
                    onClick={() => toggle(ev.id)}
                  >
                    <td className="table-cell">
                      {expandedId === ev.id
                        ? <ChevronUp className="w-3.5 h-3.5 text-slate-400" />
                        : <ChevronDown className="w-3.5 h-3.5 text-slate-400" />}
                    </td>
                    <td className="table-cell text-xs metric-font text-slate-500">
                      {fmtTime(ev.timestamp)}
                    </td>
                    <td className="table-cell text-xs font-medium text-slate-700 max-w-[150px] truncate">
                      {ev.agent_id || '—'}
                    </td>
                    <td className="table-cell text-xs text-slate-600">{ev.model || '—'}</td>
                    <td className="table-cell text-xs text-slate-500">{ev.provider || '—'}</td>
                    <td className="table-cell text-xs metric-font text-right text-slate-600">
                      {ev.input_tokens != null && ev.output_tokens != null
                        ? (ev.input_tokens + ev.output_tokens).toLocaleString()
                        : '—'}
                    </td>
                    <td className="table-cell text-xs metric-font text-right text-slate-600">
                      {ev.cost_usd != null ? `$${Number(ev.cost_usd).toFixed(5)}` : '—'}
                    </td>
                    <td className="table-cell text-xs metric-font text-right text-slate-600">
                      {ev.latency_ms != null ? `${ev.latency_ms}ms` : '—'}
                    </td>
                    <td className="table-cell text-center">
                      <span className={statusBadge(ev.status_code)}>
                        {ev.status_code ?? '—'}
                      </span>
                    </td>
                  </tr>
                  {expandedId === ev.id && <ExpandedRow ev={ev} />}
                </React.Fragment>
              ))}
            </tbody>
          </table>

          {events.length === 0 && !isLoading && (
            <div className="text-center py-12 text-slate-400">
              <Radio className="w-10 h-10 mx-auto mb-3 text-slate-300" />
              <p className="text-sm font-medium">No events found</p>
              <p className="text-xs mt-1">Route AI requests through the proxy on port 4000</p>
            </div>
          )}
        </div>

        {/* Pagination */}
        <div className="flex items-center justify-between">
          <span className="text-xs text-slate-400">
            Showing {page * PAGE_SIZE + 1}–{page * PAGE_SIZE + events.length}
            {events.length === PAGE_SIZE ? ' (more available)' : ''}
          </span>
          <div className="flex items-center gap-2">
            <button
              className="btn-secondary text-xs py-1.5"
              disabled={page === 0}
              onClick={() => setPage(p => p - 1)}
            >
              Previous
            </button>
            <button
              className="btn-secondary text-xs py-1.5"
              disabled={events.length < PAGE_SIZE}
              onClick={() => setPage(p => p + 1)}
            >
              Next
            </button>
          </div>
        </div>

      </div>
    </div>
  )
}
