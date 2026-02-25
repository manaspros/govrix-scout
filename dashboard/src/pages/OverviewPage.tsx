import { useMemo } from 'react'
import { Link } from 'react-router-dom'
import {
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from 'recharts'
import { format, parseISO } from 'date-fns'
import { Bot, Activity, DollarSign, Gauge, ArrowRight } from 'lucide-react'
import { useAgents, useEvents, useCostSummary, useCostBreakdown } from '@/api/hooks'
import { StatusBadge } from '@/components/common/StatusBadge'
import { CardSkeleton, ChartSkeleton } from '@/components/common/LoadingState'
import type { AgentEvent } from '@/api/types'

// ── Stat card ─────────────────────────────────────────────────────────────────

interface StatCardProps {
  label: string
  value: string | number
  sub?: string
  icon: React.ReactNode
  trend?: 'up' | 'down' | 'neutral'
}

function StatCard({ label, value, sub, icon }: StatCardProps) {
  return (
    <div className="bg-slate-800 rounded-xl p-5 border border-slate-700/60 flex items-start gap-4">
      <div className="flex items-center justify-center w-10 h-10 rounded-lg bg-slate-700/60 shrink-0">
        {icon}
      </div>
      <div className="min-w-0">
        <div className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-1">{label}</div>
        <div className="text-2xl font-bold text-white tabular-nums">{value}</div>
        {sub && <div className="text-xs text-slate-500 mt-0.5">{sub}</div>}
      </div>
    </div>
  )
}

// ── Chart tooltip ─────────────────────────────────────────────────────────────

interface ChartTooltipPayload {
  name: string
  value: number
  color: string
}

interface CustomTooltipProps {
  active?: boolean
  payload?: ChartTooltipPayload[]
  label?: string
  formatter?: (v: number) => string
}

function CustomTooltip({ active, payload, label, formatter }: CustomTooltipProps) {
  if (!active || !payload?.length) return null
  return (
    <div className="bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 shadow-xl text-xs">
      <div className="text-slate-400 mb-1">{label}</div>
      {payload.map(p => (
        <div key={p.name} className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full" style={{ background: p.color }} />
          <span className="text-slate-300 capitalize">{p.name}:</span>
          <span className="text-white font-semibold">{formatter ? formatter(p.value) : p.value}</span>
        </div>
      ))}
    </div>
  )
}

// ── Recent events row ─────────────────────────────────────────────────────────

function EventRow({ event }: { event: AgentEvent }) {
  return (
    <tr className="hover:bg-slate-700/20 transition-colors">
      <td className="px-4 py-2.5 text-xs text-slate-400 font-mono whitespace-nowrap">
        {format(parseISO(event.timestamp), 'HH:mm:ss')}
      </td>
      <td className="px-4 py-2.5 text-xs text-slate-300 max-w-[120px] truncate">
        {event.agent_id}
      </td>
      <td className="px-4 py-2.5">
        <StatusBadge value={event.kind} />
      </td>
      <td className="px-4 py-2.5">
        <StatusBadge value={event.protocol} />
      </td>
      <td className="px-4 py-2.5 text-xs text-slate-300">{event.model ?? '—'}</td>
      <td className="px-4 py-2.5 text-xs text-slate-400 tabular-nums text-right">
        {event.total_tokens?.toLocaleString() ?? '—'}
      </td>
      <td className="px-4 py-2.5 text-xs text-slate-400 tabular-nums text-right">
        {event.latency_ms != null ? `${event.latency_ms}ms` : '—'}
      </td>
    </tr>
  )
}

// ── Overview page ─────────────────────────────────────────────────────────────

export function OverviewPage() {
  const { data: agentsData, isLoading: agentsLoading } = useAgents()
  const { data: eventsData, isLoading: eventsLoading } = useEvents({ limit: 10 }, true)
  const { data: costSummary, isLoading: costLoading } = useCostSummary({ days: 1 })
  const { data: costBreakdown, isLoading: chartLoading } = useCostBreakdown({ days: 7 })

  // Build daily requests chart from cost breakdown daily data
  const dailyData = useMemo(() => {
    if (!costBreakdown?.daily) return []
    return costBreakdown.daily.map(d => ({
      date: format(parseISO(d.timestamp), 'MMM d'),
      requests: d.requests,
      cost: d.cost_usd,
    }))
  }, [costBreakdown])

  // Top 5 agents by cost
  const topAgents = useMemo(() => {
    if (!costBreakdown?.by_agent) return []
    return [...costBreakdown.by_agent]
      .sort((a, b) => b.cost_usd - a.cost_usd)
      .slice(0, 5)
      .map(a => ({ name: a.group.slice(0, 12), cost: a.cost_usd }))
  }, [costBreakdown])

  const avgLatency = useMemo(() => {
    const events = eventsData?.events ?? []
    const withLatency = events.filter(e => e.latency_ms != null)
    if (!withLatency.length) return null
    const avg = withLatency.reduce((s, e) => s + (e.latency_ms ?? 0), 0) / withLatency.length
    return Math.round(avg)
  }, [eventsData])

  return (
    <div className="space-y-6">
      {/* Stat cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
        {agentsLoading ? (
          <>
            <CardSkeleton /><CardSkeleton /><CardSkeleton /><CardSkeleton />
          </>
        ) : (
          <>
            <StatCard
              label="Total Agents"
              value={agentsData?.total ?? 0}
              sub="Registered and active"
              icon={<Bot className="w-5 h-5 text-brand-400" />}
            />
            <StatCard
              label="Events (24h)"
              value={(eventsData?.total ?? 0).toLocaleString()}
              sub="Intercepted API calls"
              icon={<Activity className="w-5 h-5 text-violet-400" />}
            />
            <StatCard
              label="Cost (24h)"
              value={costLoading ? '…' : `$${(costSummary?.total_cost_usd ?? 0).toFixed(4)}`}
              sub={`${(costSummary?.total_tokens ?? 0).toLocaleString()} tokens`}
              icon={<DollarSign className="w-5 h-5 text-emerald-400" />}
            />
            <StatCard
              label="Avg Latency"
              value={avgLatency != null ? `${avgLatency}ms` : '—'}
              sub="Proxy overhead (last 10 events)"
              icon={<Gauge className="w-5 h-5 text-orange-400" />}
            />
          </>
        )}
      </div>

      {/* Charts row */}
      <div className="grid grid-cols-1 xl:grid-cols-3 gap-4">
        {/* Requests over time */}
        <div className="xl:col-span-2 bg-slate-800 rounded-xl border border-slate-700/60 p-5">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-slate-200">Requests (7 days)</h2>
          </div>
          {chartLoading ? (
            <ChartSkeleton height={180} />
          ) : (
            <ResponsiveContainer width="100%" height={180}>
              <AreaChart data={dailyData} margin={{ top: 2, right: 4, left: -20, bottom: 0 }}>
                <defs>
                  <linearGradient id="reqGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="#0ea5e9" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="#0ea5e9" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                <XAxis dataKey="date" tick={{ fontSize: 11, fill: '#94a3b8' }} />
                <YAxis tick={{ fontSize: 11, fill: '#94a3b8' }} />
                <Tooltip content={<CustomTooltip />} />
                <Area
                  type="monotone"
                  dataKey="requests"
                  stroke="#0ea5e9"
                  strokeWidth={2}
                  fill="url(#reqGrad)"
                  dot={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          )}
        </div>

        {/* Cost by agent */}
        <div className="bg-slate-800 rounded-xl border border-slate-700/60 p-5">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-slate-200">Cost by Agent (top 5)</h2>
          </div>
          {chartLoading ? (
            <ChartSkeleton height={180} />
          ) : (
            <ResponsiveContainer width="100%" height={180}>
              <BarChart data={topAgents} margin={{ top: 2, right: 4, left: -20, bottom: 0 }}>
                <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
                <XAxis dataKey="name" tick={{ fontSize: 10, fill: '#94a3b8' }} />
                <YAxis tick={{ fontSize: 10, fill: '#94a3b8' }} />
                <Tooltip
                  content={<CustomTooltip formatter={v => `$${v.toFixed(4)}`} />}
                />
                <Bar dataKey="cost" fill="#10b981" radius={[3, 3, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          )}
        </div>
      </div>

      {/* Recent events */}
      <div className="bg-slate-800 rounded-xl border border-slate-700/60">
        <div className="flex items-center justify-between px-5 py-4 border-b border-slate-700/60">
          <h2 className="text-sm font-semibold text-slate-200">Recent Events</h2>
          <Link
            to="/events"
            className="flex items-center gap-1 text-xs text-brand-400 hover:text-brand-300 transition-colors"
          >
            View all <ArrowRight className="w-3 h-3" />
          </Link>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead className="bg-slate-800/60">
              <tr>
                {['Time', 'Agent', 'Kind', 'Protocol', 'Model', 'Tokens', 'Latency'].map(h => (
                  <th
                    key={h}
                    className="px-4 py-2.5 text-left text-[11px] font-semibold text-slate-500 uppercase tracking-wider"
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-700/30">
              {eventsLoading ? (
                Array.from({ length: 5 }).map((_, i) => (
                  <tr key={i} className="animate-pulse">
                    {Array.from({ length: 7 }).map((__, j) => (
                      <td key={j} className="px-4 py-3">
                        <div className="h-3 bg-slate-700/50 rounded" />
                      </td>
                    ))}
                  </tr>
                ))
              ) : (eventsData?.events ?? []).slice(0, 10).map(e => (
                <EventRow key={e.id} event={e} />
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}
