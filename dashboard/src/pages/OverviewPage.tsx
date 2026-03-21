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
import { EventStream } from '@/components/EventStream'
import type { AgentEvent } from '@/api/types'

// ── Stat card ─────────────────────────────────────────────────────────────────

interface StatCardProps {
  label: string
  value: string | number
  sub?: string
  icon: React.ReactNode
  accentColor?: string
  iconBg?: string
}

function StatCard({
  label,
  value,
  sub,
  icon,
  accentColor = '#10b981',
  iconBg = 'rgba(16,185,129,0.1)',
}: StatCardProps) {
  return (
    <div className="glass-card relative overflow-hidden p-5 group">
      {/* Accent gradient line at top */}
      <div
        className="absolute top-0 left-5 right-5 h-px"
        style={{
          background: `linear-gradient(90deg, transparent, ${accentColor}70, transparent)`,
        }}
      />
      <div className="flex items-start gap-4">
        {/* Icon */}
        <div
          className="flex items-center justify-center w-11 h-11 rounded-xl shrink-0 mt-0.5 transition-transform duration-300 group-hover:scale-105"
          style={{ background: iconBg, border: `1px solid ${accentColor}22` }}
        >
          {icon}
        </div>
        <div className="min-w-0">
          <div className="section-label mb-1.5">{label}</div>
          <div className="stat-value text-[1.875rem] text-white tabular-nums">{value}</div>
          {sub && <div className="text-xs text-slate-500 mt-1.5 leading-snug">{sub}</div>}
        </div>
      </div>
      {/* Corner bloom */}
      <div
        className="absolute -bottom-6 -right-6 w-24 h-24 rounded-full pointer-events-none"
        style={{ background: accentColor, opacity: 0.05, filter: 'blur(24px)' }}
      />
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
    <div
      style={{
        background: 'rgba(6, 10, 19, 0.92)',
        backdropFilter: 'blur(16px)',
        WebkitBackdropFilter: 'blur(16px)',
        border: '1px solid rgba(148, 163, 184, 0.12)',
        borderTop: '1px solid rgba(255, 255, 255, 0.07)',
        borderRadius: '10px',
        padding: '10px 14px',
        boxShadow: '0 8px 32px rgba(0,0,0,0.5)',
      }}
    >
      <div
        className="text-[10px] text-slate-500 uppercase tracking-wider mb-2"
        style={{ fontFamily: 'JetBrains Mono' }}
      >
        {label}
      </div>
      {payload.map(p => (
        <div key={p.name} className="flex items-center gap-2.5 min-w-[130px]">
          <span
            className="w-1.5 h-1.5 rounded-full shrink-0"
            style={{ background: p.color }}
          />
          <span className="text-slate-400 capitalize text-xs">{p.name}</span>
          <span
            className="text-white font-semibold text-xs tabular-nums ml-auto pl-4"
            style={{ fontFamily: 'JetBrains Mono' }}
          >
            {formatter ? formatter(p.value) : p.value.toLocaleString()}
          </span>
        </div>
      ))}
    </div>
  )
}

// ── Recent events row ─────────────────────────────────────────────────────────

function EventRow({ event }: { event: AgentEvent }) {
  return (
    <tr className="hover:bg-white/[0.02] transition-colors">
      <td
        className="px-4 py-2.5 text-xs text-slate-500 whitespace-nowrap"
        style={{ fontFamily: 'JetBrains Mono' }}
      >
        {format(parseISO(event.timestamp), 'HH:mm:ss')}
      </td>
      <td className="px-4 py-2.5 text-xs text-slate-200 max-w-[120px] truncate font-medium">
        {event.agent_id}
      </td>
      <td className="px-4 py-2.5">
        <StatusBadge value={event.kind} />
      </td>
      <td className="px-4 py-2.5">
        <StatusBadge value={event.protocol} />
      </td>
      <td className="px-4 py-2.5 text-xs text-slate-300">{event.model ?? '—'}</td>
      <td
        className="px-4 py-2.5 text-xs text-slate-400 tabular-nums text-right"
        style={{ fontFamily: 'JetBrains Mono' }}
      >
        {event.total_tokens?.toLocaleString() ?? '—'}
      </td>
      <td
        className="px-4 py-2.5 text-xs text-slate-400 tabular-nums text-right"
        style={{ fontFamily: 'JetBrains Mono' }}
      >
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

  const dailyData = useMemo(() => {
    if (!costBreakdown?.daily) return []
    return costBreakdown.daily.map(d => ({
      date: format(parseISO(d.timestamp), 'MMM d'),
      requests: d.requests,
      cost: d.cost_usd,
    }))
  }, [costBreakdown])

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

  const GRID_COLOR = 'rgba(148,163,184,0.06)'
  const AXIS_TICK = {
    fontSize: 11,
    fill: '#3f5068',
    fontFamily: 'JetBrains Mono',
  }

  return (
    <div className="space-y-5 stagger-in">
      {/* Stat cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
        {agentsLoading ? (
          <><CardSkeleton /><CardSkeleton /><CardSkeleton /><CardSkeleton /></>
        ) : (
          <>
            <StatCard
              label="Total Agents"
              value={agentsData?.total ?? 0}
              sub="Registered and active"
              icon={<Bot className="w-5 h-5 text-brand-400" />}
              accentColor="#10b981"
              iconBg="rgba(16,185,129,0.1)"
            />
            <StatCard
              label="Events (24h)"
              value={(eventsData?.total ?? 0).toLocaleString()}
              sub="Intercepted API calls"
              icon={<Activity className="w-5 h-5 text-violet-400" />}
              accentColor="#8b5cf6"
              iconBg="rgba(139,92,246,0.1)"
            />
            <StatCard
              label="Cost (24h)"
              value={costLoading ? '…' : `$${(costSummary?.total_cost_usd ?? 0).toFixed(4)}`}
              sub={`${(costSummary?.total_tokens ?? 0).toLocaleString()} tokens`}
              icon={<DollarSign className="w-5 h-5 text-emerald-400" />}
              accentColor="#34d399"
              iconBg="rgba(52,211,153,0.08)"
            />
            <StatCard
              label="Avg Latency"
              value={avgLatency != null ? `${avgLatency}ms` : '—'}
              sub="Proxy overhead (last 10 events)"
              icon={<Gauge className="w-5 h-5 text-orange-400" />}
              accentColor="#f97316"
              iconBg="rgba(249,115,22,0.1)"
            />
          </>
        )}
      </div>

      {/* Charts row */}
      <div className="grid grid-cols-1 xl:grid-cols-3 gap-4">
        {/* Requests over time */}
        <div className="xl:col-span-2 glass-card p-5">
          <div className="flex items-center justify-between mb-5">
            <div>
              <h2 className="text-sm font-semibold text-slate-200 font-display">Requests</h2>
              <p className="text-[11px] text-slate-600 mt-0.5">7-day volume</p>
            </div>
          </div>
          {chartLoading ? (
            <ChartSkeleton height={180} />
          ) : (
            <ResponsiveContainer width="100%" height={180}>
              <AreaChart data={dailyData} margin={{ top: 4, right: 4, left: -18, bottom: 0 }}>
                <defs>
                  <linearGradient id="govrix-reqGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#0ea5e9" stopOpacity={0.28} />
                    <stop offset="75%" stopColor="#0ea5e9" stopOpacity={0.04} />
                    <stop offset="100%" stopColor="#0ea5e9" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid
                  strokeDasharray="4 4"
                  stroke={GRID_COLOR}
                  vertical={false}
                />
                <XAxis
                  dataKey="date"
                  tick={AXIS_TICK}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis
                  tick={AXIS_TICK}
                  axisLine={false}
                  tickLine={false}
                />
                <Tooltip
                  content={<CustomTooltip />}
                  cursor={{ stroke: 'rgba(148,163,184,0.1)', strokeWidth: 1 }}
                />
                <Area
                  type="monotone"
                  dataKey="requests"
                  stroke="#0ea5e9"
                  strokeWidth={2}
                  fill="url(#govrix-reqGrad)"
                  dot={false}
                  activeDot={{ r: 4, fill: '#0ea5e9', strokeWidth: 0 }}
                />
              </AreaChart>
            </ResponsiveContainer>
          )}
        </div>

        {/* Cost by agent */}
        <div className="glass-card p-5">
          <div className="flex items-center justify-between mb-5">
            <div>
              <h2 className="text-sm font-semibold text-slate-200 font-display">Cost by Agent</h2>
              <p className="text-[11px] text-slate-600 mt-0.5">Top 5 this week</p>
            </div>
          </div>
          {chartLoading ? (
            <ChartSkeleton height={180} />
          ) : (
            <ResponsiveContainer width="100%" height={180}>
              <BarChart data={topAgents} margin={{ top: 4, right: 4, left: -18, bottom: 0 }} barSize={14}>
                <defs>
                  <linearGradient id="govrix-costGrad" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#10b981" stopOpacity={0.9} />
                    <stop offset="100%" stopColor="#10b981" stopOpacity={0.3} />
                  </linearGradient>
                </defs>
                <CartesianGrid
                  strokeDasharray="4 4"
                  stroke={GRID_COLOR}
                  vertical={false}
                />
                <XAxis
                  dataKey="name"
                  tick={{ ...AXIS_TICK, fontSize: 10 }}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis
                  tick={{ ...AXIS_TICK, fontSize: 10 }}
                  axisLine={false}
                  tickLine={false}
                />
                <Tooltip
                  content={<CustomTooltip formatter={v => `$${v.toFixed(4)}`} />}
                  cursor={{ fill: 'rgba(255,255,255,0.03)' }}
                />
                <Bar dataKey="cost" fill="url(#govrix-costGrad)" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          )}
        </div>
      </div>

      {/* Live stream panel */}
      <EventStream maxHeight={280} />

      {/* Recent events */}
      <div className="glass-card overflow-hidden">
        <div
          className="flex items-center justify-between px-5 py-4"
          style={{ borderBottom: '1px solid rgba(148,163,184,0.07)' }}
        >
          <div>
            <h2 className="text-sm font-semibold text-slate-200 font-display">Recent Events</h2>
            <p className="text-[11px] text-slate-600 mt-0.5">Live stream — last 10</p>
          </div>
          <Link
            to="/events"
            className="flex items-center gap-1.5 text-xs text-brand-400 hover:text-brand-300 transition-colors font-medium"
          >
            View all <ArrowRight className="w-3.5 h-3.5" />
          </Link>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr style={{ background: 'rgba(255,255,255,0.015)' }}>
                {['Time', 'Agent', 'Kind', 'Protocol', 'Model', 'Tokens', 'Latency'].map(h => (
                  <th
                    key={h}
                    className="px-4 py-2.5 text-left"
                    style={{
                      fontSize: '10px',
                      fontFamily: 'Sora, sans-serif',
                      fontWeight: 600,
                      letterSpacing: '0.08em',
                      textTransform: 'uppercase',
                      color: '#475569',
                      borderBottom: '1px solid rgba(148,163,184,0.07)',
                    }}
                  >
                    {h}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody style={{ borderColor: 'rgba(148,163,184,0.05)' }}>
              {eventsLoading ? (
                Array.from({ length: 5 }).map((_, i) => (
                  <tr key={i} className="animate-pulse">
                    {Array.from({ length: 7 }).map((__, j) => (
                      <td key={j} className="px-4 py-3">
                        <div className="h-3 rounded skeleton" />
                      </td>
                    ))}
                  </tr>
                ))
              ) : (
                (eventsData?.events ?? []).slice(0, 10).map(e => (
                  <EventRow key={e.id} event={e} />
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  )
}
