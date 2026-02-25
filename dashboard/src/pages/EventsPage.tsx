import { useState } from 'react'
import { format, parseISO } from 'date-fns'
import { Activity, RefreshCw, X } from 'lucide-react'
import { useEvents } from '@/api/hooks'
import { StatusBadge } from '@/components/common/StatusBadge'
import { EmptyState } from '@/components/common/EmptyState'
import { JsonViewer } from '@/components/common/JsonViewer'
import { TimeRangePicker, timeRangeToSince } from '@/components/common/TimeRangePicker'
import type { TimeRange } from '@/components/common/TimeRangePicker'
import type { AgentEvent } from '@/api/types'

// ── Event detail drawer ───────────────────────────────────────────────────────

interface EventDetailProps {
  event: AgentEvent
  onClose: () => void
}

function EventDetail({ event, onClose }: EventDetailProps) {
  return (
    <div className="fixed inset-y-0 right-0 w-full max-w-lg bg-slate-900 border-l border-slate-700 flex flex-col shadow-2xl z-50">
      <div className="flex items-center justify-between px-5 py-4 border-b border-slate-700 shrink-0">
        <div>
          <div className="text-sm font-semibold text-slate-100">Event Detail</div>
          <div className="text-xs text-slate-500 font-mono mt-0.5">{event.id}</div>
        </div>
        <button
          onClick={onClose}
          className="p-1.5 rounded-lg text-slate-400 hover:text-slate-200 hover:bg-slate-700 transition-colors"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-5 space-y-4">
        {/* Core fields */}
        <div className="grid grid-cols-2 gap-3 text-xs">
          {[
            ['Session', event.session_id],
            ['Agent', event.agent_id],
            ['Kind', event.kind],
            ['Protocol', event.protocol],
            ['Model', event.model ?? '—'],
            ['Status', event.status_code?.toString() ?? '—'],
            ['Finish Reason', event.finish_reason ?? '—'],
            ['Upstream', event.upstream_target],
          ].map(([label, value]) => (
            <div key={label} className="bg-slate-800 rounded-lg p-2.5">
              <div className="text-slate-500 uppercase tracking-wider mb-1">{label}</div>
              <div className="text-slate-200 font-mono break-all">{value}</div>
            </div>
          ))}
        </div>

        {/* Token / cost / latency */}
        <div className="grid grid-cols-3 gap-3 text-xs">
          <div className="bg-slate-800 rounded-lg p-2.5 text-center">
            <div className="text-slate-500 mb-1">Input tokens</div>
            <div className="text-slate-200 font-bold tabular-nums">{event.input_tokens?.toLocaleString() ?? '—'}</div>
          </div>
          <div className="bg-slate-800 rounded-lg p-2.5 text-center">
            <div className="text-slate-500 mb-1">Output tokens</div>
            <div className="text-slate-200 font-bold tabular-nums">{event.output_tokens?.toLocaleString() ?? '—'}</div>
          </div>
          <div className="bg-slate-800 rounded-lg p-2.5 text-center">
            <div className="text-slate-500 mb-1">Latency</div>
            <div className="text-slate-200 font-bold tabular-nums">
              {event.latency_ms != null ? `${event.latency_ms}ms` : '—'}
            </div>
          </div>
          <div className="bg-slate-800 rounded-lg p-2.5 text-center col-span-2">
            <div className="text-slate-500 mb-1">Cost</div>
            <div className="text-emerald-400 font-bold tabular-nums">
              {event.cost_usd != null ? `$${event.cost_usd.toFixed(8)}` : '—'}
            </div>
          </div>
          <div className="bg-slate-800 rounded-lg p-2.5 text-center">
            <div className="text-slate-500 mb-1">Raw size</div>
            <div className="text-slate-200 font-bold tabular-nums">
              {event.raw_size_bytes != null ? `${event.raw_size_bytes}B` : '—'}
            </div>
          </div>
        </div>

        {/* Timestamp */}
        <div className="text-xs text-slate-500">
          <span className="text-slate-400 font-medium">Timestamp: </span>
          {format(parseISO(event.timestamp), 'PPPppp')}
        </div>

        {/* Error */}
        {event.error_message && (
          <div className="bg-red-900/20 border border-red-700/40 rounded-lg p-3 text-xs text-red-400">
            <div className="font-semibold mb-1">Error</div>
            {event.error_message}
          </div>
        )}

        {/* Payload */}
        <JsonViewer data={event.payload} collapsed={false} />

        {/* Tags */}
        {Object.keys(event.tags ?? {}).length > 0 && (
          <div>
            <div className="text-xs text-slate-500 uppercase tracking-wider mb-2">Tags</div>
            <JsonViewer data={event.tags} collapsed={false} />
          </div>
        )}
      </div>
    </div>
  )
}

// ── Events page ───────────────────────────────────────────────────────────────

export function EventsPage() {
  const [agentFilter, setAgentFilter] = useState('')
  const [kindFilter, setKindFilter] = useState('')
  const [protocolFilter, setProtocolFilter] = useState('')
  const [modelFilter, setModelFilter] = useState('')
  const [timeRange, setTimeRange] = useState<TimeRange>('24h')
  const [autoRefresh, setAutoRefresh] = useState(false)
  const [selectedEvent, setSelectedEvent] = useState<AgentEvent | null>(null)

  const { data, isLoading, refetch, isFetching } = useEvents(
    {
      agent_id: agentFilter || undefined,
      kind: kindFilter || undefined,
      protocol: protocolFilter || undefined,
      model: modelFilter || undefined,
      since: timeRangeToSince(timeRange),
      limit: 100,
    },
    autoRefresh,
  )

  const events = data?.events ?? []

  return (
    <div className="space-y-4">
      {/* Toolbar */}
      <div className="flex flex-col gap-3">
        <div className="flex flex-wrap gap-2 items-center">
          {/* Filters */}
          {[
            { placeholder: 'Agent ID', value: agentFilter, onChange: setAgentFilter },
            { placeholder: 'Kind (llm, tool…)', value: kindFilter, onChange: setKindFilter },
            { placeholder: 'Protocol', value: protocolFilter, onChange: setProtocolFilter },
            { placeholder: 'Model', value: modelFilter, onChange: setModelFilter },
          ].map(f => (
            <input
              key={f.placeholder}
              type="text"
              placeholder={f.placeholder}
              value={f.value}
              onChange={e => f.onChange(e.target.value)}
              className="px-3 py-2 text-sm bg-slate-800 border border-slate-600 rounded-lg text-slate-200 placeholder-slate-500 focus:outline-none focus:ring-1 focus:ring-brand-500 w-40"
            />
          ))}

          <TimeRangePicker value={timeRange} onChange={setTimeRange} />

          {/* Auto-refresh toggle */}
          <button
            onClick={() => setAutoRefresh(r => !r)}
            className={`flex items-center gap-1.5 px-3 py-2 text-xs rounded-lg border transition-colors ${
              autoRefresh
                ? 'bg-brand-600/20 border-brand-600/40 text-brand-400'
                : 'bg-slate-800 border-slate-600 text-slate-400 hover:text-slate-200'
            }`}
          >
            <RefreshCw className={`w-3.5 h-3.5 ${autoRefresh ? 'animate-spin' : ''}`} />
            {autoRefresh ? 'Live' : 'Auto-refresh'}
          </button>

          <button
            onClick={() => void refetch()}
            disabled={isFetching}
            className="flex items-center gap-1.5 px-3 py-2 text-xs bg-slate-700 text-slate-300 rounded-lg hover:bg-slate-600 transition-colors disabled:opacity-50"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isFetching ? 'animate-spin' : ''}`} />
            Refresh
          </button>

          <span className="text-xs text-slate-500 ml-auto">{data?.total ?? 0} total</span>
        </div>
      </div>

      {/* Table */}
      <div className="overflow-x-auto rounded-xl border border-slate-700">
        <table className="w-full text-sm">
          <thead className="bg-slate-800/80">
            <tr>
              {['Timestamp', 'Agent', 'Kind', 'Protocol', 'Model', 'Tokens', 'Cost', 'Latency', 'Status'].map(h => (
                <th key={h} className="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider whitespace-nowrap">
                  {h}
                </th>
              ))}
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-700/40">
            {isLoading ? (
              Array.from({ length: 8 }).map((_, i) => (
                <tr key={i} className="animate-pulse">
                  {Array.from({ length: 9 }).map((__, j) => (
                    <td key={j} className="px-4 py-3">
                      <div className="h-4 bg-slate-700/50 rounded" />
                    </td>
                  ))}
                </tr>
              ))
            ) : events.length === 0 ? (
              <tr>
                <td colSpan={9}>
                  <EmptyState
                    icon={Activity}
                    title="No events"
                    description="Events will appear here as agents make API calls through the proxy."
                  />
                </td>
              </tr>
            ) : (
              events.map(event => (
                <tr
                  key={event.id}
                  onClick={() => setSelectedEvent(event)}
                  className="cursor-pointer hover:bg-slate-700/25 transition-colors"
                >
                  <td className="px-4 py-2.5 text-xs text-slate-400 font-mono whitespace-nowrap">
                    {format(parseISO(event.timestamp), 'MMM d HH:mm:ss')}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-slate-300 max-w-[120px] truncate font-mono">
                    {event.agent_id}
                  </td>
                  <td className="px-4 py-2.5"><StatusBadge value={event.kind} /></td>
                  <td className="px-4 py-2.5"><StatusBadge value={event.protocol} /></td>
                  <td className="px-4 py-2.5 text-xs text-slate-400 max-w-[120px] truncate">
                    {event.model ?? '—'}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-slate-400 tabular-nums text-right">
                    {event.total_tokens?.toLocaleString() ?? '—'}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-slate-400 tabular-nums text-right">
                    {event.cost_usd != null ? `$${event.cost_usd.toFixed(6)}` : '—'}
                  </td>
                  <td className="px-4 py-2.5 text-xs text-slate-400 tabular-nums text-right whitespace-nowrap">
                    {event.latency_ms != null ? `${event.latency_ms}ms` : '—'}
                  </td>
                  <td className="px-4 py-2.5">
                    {event.status_code != null && (
                      <StatusBadge
                        value={String(event.status_code)}
                        variant={
                          event.status_code >= 500 ? 'error' :
                          event.status_code >= 400 ? 'blocked' :
                          'active'
                        }
                      />
                    )}
                  </td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>

      {/* Event detail overlay */}
      {selectedEvent && (
        <>
          <div
            className="fixed inset-0 bg-black/50 z-40"
            onClick={() => setSelectedEvent(null)}
          />
          <EventDetail event={selectedEvent} onClose={() => setSelectedEvent(null)} />
        </>
      )}
    </div>
  )
}
