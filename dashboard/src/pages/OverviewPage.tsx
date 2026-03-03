import {
  BarChart, Bar, PieChart, Pie, Cell,
  XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid,
} from 'recharts'
import { Activity, Bot, DollarSign, Zap, Clock } from 'lucide-react'
import { useCostSummary, useCostBreakdown, useEvents, useAgents } from '../api/hooks'
import type { ElementType } from 'react'

const COLORS = ['#6366f1', '#8b5cf6', '#0ea5e9', '#10b981', '#f59e0b', '#f43f5e', '#64748b']

const fmt = (n: number | undefined | null): string =>
  typeof n === 'number' ? n.toLocaleString() : '—'

const fmtUsd = (n: number | undefined | null): string =>
  typeof n === 'number' ? `$${n.toFixed(4)}` : '$0.00'

interface StatCardProps {
  icon: ElementType
  label: string
  value: string
  sub?: string
  color?: string
}

const StatCard = ({ icon: Icon, label, value, sub, color = 'text-primary' }: StatCardProps) => (
  <div className="stat-card">
    <div className="flex items-center justify-between mb-3">
      <span className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">{label}</span>
      <div className={`w-8 h-8 rounded-lg ${
        color === 'text-primary'
          ? 'bg-indigo-50'
          : color === 'text-emerald-600'
          ? 'bg-emerald-50'
          : color === 'text-amber-600'
          ? 'bg-amber-50'
          : 'bg-slate-50'
      } flex items-center justify-center`}>
        <Icon className={`w-4 h-4 ${color}`} />
      </div>
    </div>
    <div className="text-3xl font-black text-slate-900 metric-font tracking-tight">{value}</div>
    {sub && <p className="text-xs text-slate-400 mt-1">{sub}</p>}
  </div>
)

export default function OverviewPage() {
  const { data: costData } = useCostSummary()
  const { data: breakdownData } = useCostBreakdown()
  const { data: eventsData } = useEvents({ limit: 10 })
  const { data: agentsData } = useAgents()

  const summary = costData ?? {}
  const modelRows = (breakdownData?.by_model ?? []).slice(0, 7)
  const agentRows = (breakdownData?.by_agent ?? []).slice(0, 7)
  const events = eventsData?.data ?? []
  const agents = agentsData?.data ?? []

  // Bar chart data for model costs (replacing timeseries AreaChart)
  const modelBarData = modelRows.map(r => ({
    name: r.label || 'unknown',
    cost: r.cost_usd || 0,
    requests: r.requests || 0,
  }))

  // Cost by agent bar chart data
  const agentBarData = agentRows.map(r => ({
    group_key: r.label || 'unknown',
    total_cost_usd: r.cost_usd || 0,
  }))

  // Pie data for model distribution
  const pieData = modelRows.map(r => ({
    name: r.label || 'unknown',
    value: r.requests || 0,
  }))

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-[1400px] mx-auto space-y-6">

        {/* KPI Cards */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <StatCard
            icon={Activity}
            label="Total Requests"
            value={fmt((summary as { total_requests?: number }).total_requests)}
            sub="All time"
          />
          <StatCard
            icon={DollarSign}
            label="Total Cost"
            value={fmtUsd((summary as { total_cost_usd?: number }).total_cost_usd)}
            sub="All time"
            color="text-emerald-600"
          />
          <StatCard
            icon={Clock}
            label="Avg Cost / Req"
            value={fmtUsd((summary as { avg_cost_per_request?: number }).avg_cost_per_request)}
            sub={`${fmt((summary as { total_input_tokens?: number }).total_input_tokens)} input tokens`}
            color="text-amber-600"
          />
          <StatCard
            icon={Bot}
            label="Active Agents"
            value={fmt(Array.isArray(agents) ? agents.length : 0)}
            sub={`${fmt((summary as { total_output_tokens?: number }).total_output_tokens)} output tokens`}
          />
        </div>

        {/* Charts Row */}
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
          {/* Cost by Model Bar Chart (replacing timeseries AreaChart — no timeseries API) */}
          <div className="lg:col-span-2 bg-white border border-slate-200 rounded-xl p-5">
            <h3 className="text-sm font-bold text-slate-700 mb-4">Cost by Model</h3>
            {modelBarData.length > 0 ? (
              <ResponsiveContainer width="100%" height={220}>
                <BarChart data={modelBarData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
                  <XAxis
                    dataKey="name"
                    tick={{ fontSize: 10, fill: '#94a3b8' }}
                    axisLine={false}
                    tickLine={false}
                  />
                  <YAxis
                    tick={{ fontSize: 10, fill: '#94a3b8' }}
                    axisLine={false}
                    tickLine={false}
                    width={55}
                    tickFormatter={(v: number) => `$${v.toFixed(4)}`}
                  />
                  <Tooltip
                    contentStyle={{ fontSize: 12, borderRadius: 8, border: '1px solid #e2e8f0' }}
                    formatter={(v: number) => [`$${v.toFixed(5)}`, 'Cost (USD)']}
                  />
                  <Bar dataKey="cost" fill="#6366f1" radius={[4, 4, 0, 0]} name="Cost (USD)" />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="h-[220px] flex items-center justify-center text-slate-400 text-sm">
                <Zap className="w-6 h-6 mr-2 text-slate-300" />
                No cost data yet — route AI requests through the proxy
              </div>
            )}
          </div>

          {/* Model Distribution Donut */}
          <div className="bg-white border border-slate-200 rounded-xl p-5">
            <h3 className="text-sm font-bold text-slate-700 mb-4">Requests by Model</h3>
            {pieData.length > 0 ? (
              <ResponsiveContainer width="100%" height={220}>
                <PieChart>
                  <Pie
                    data={pieData}
                    cx="50%"
                    cy="50%"
                    innerRadius={50}
                    outerRadius={80}
                    paddingAngle={2}
                    dataKey="value"
                    nameKey="name"
                  >
                    {pieData.map((_, i) => (
                      <Cell key={i} fill={COLORS[i % COLORS.length]} />
                    ))}
                  </Pie>
                  <Tooltip contentStyle={{ fontSize: 11, borderRadius: 8 }} />
                </PieChart>
              </ResponsiveContainer>
            ) : (
              <div className="h-[220px] flex items-center justify-center text-slate-400 text-sm">No data yet</div>
            )}
            <div className="flex flex-wrap gap-2 mt-2">
              {pieData.slice(0, 5).map((d, i) => (
                <span key={i} className="text-[10px] flex items-center gap-1">
                  <span className="w-2 h-2 rounded-full" style={{ background: COLORS[i % COLORS.length] }} />
                  {d.name}
                </span>
              ))}
            </div>
          </div>
        </div>

        {/* Bottom Row: Cost by Agent + Recent Events */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {/* Cost by Agent Bar Chart */}
          <div className="bg-white border border-slate-200 rounded-xl p-5">
            <h3 className="text-sm font-bold text-slate-700 mb-4">Cost by Agent</h3>
            {agentBarData.length > 0 ? (
              <ResponsiveContainer width="100%" height={200}>
                <BarChart data={agentBarData} layout="vertical">
                  <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" horizontal={false} />
                  <XAxis
                    type="number"
                    tick={{ fontSize: 10, fill: '#94a3b8' }}
                    axisLine={false}
                    tickLine={false}
                  />
                  <YAxis
                    type="category"
                    dataKey="group_key"
                    tick={{ fontSize: 10, fill: '#64748b' }}
                    axisLine={false}
                    tickLine={false}
                    width={100}
                  />
                  <Tooltip
                    contentStyle={{ fontSize: 11, borderRadius: 8 }}
                    formatter={(v: number) => [`$${v.toFixed(4)}`, 'Cost (USD)']}
                  />
                  <Bar dataKey="total_cost_usd" fill="#6366f1" radius={[0, 4, 4, 0]} name="Cost (USD)" />
                </BarChart>
              </ResponsiveContainer>
            ) : (
              <div className="h-[200px] flex items-center justify-center text-slate-400 text-sm">No cost data yet</div>
            )}
          </div>

          {/* Recent Events */}
          <div className="bg-white border border-slate-200 rounded-xl p-5">
            <h3 className="text-sm font-bold text-slate-700 mb-4">Recent Events</h3>
            <div className="space-y-2 max-h-[200px] overflow-y-auto">
              {events.slice(0, 8).map((ev, i) => (
                <div
                  key={ev.id || i}
                  className="flex items-center justify-between py-1.5 border-b border-slate-50 last:border-0"
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <div className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
                      (ev.status_code ?? 0) >= 400 ? 'bg-red-500' : 'bg-emerald-500'
                    }`} />
                    <span className="text-xs text-slate-600 truncate">
                      {ev.model || ev.provider || 'unknown'}
                    </span>
                  </div>
                  <div className="flex items-center gap-3 flex-shrink-0">
                    <span className="text-[10px] metric-font text-slate-400">
                      {ev.input_tokens != null && ev.output_tokens != null
                        ? `${(ev.input_tokens + ev.output_tokens).toLocaleString()} tok`
                        : ''}
                    </span>
                    <span className="text-[10px] metric-font text-slate-400">
                      {ev.latency_ms != null ? `${ev.latency_ms}ms` : ''}
                    </span>
                    <span className={`badge ${(ev.status_code ?? 0) >= 400 ? 'badge-danger' : 'badge-success'}`}>
                      {ev.status_code ?? '—'}
                    </span>
                  </div>
                </div>
              ))}
              {events.length === 0 && (
                <div className="text-center text-slate-400 text-sm py-8">
                  <Zap className="w-8 h-8 mx-auto mb-2 text-slate-300" />
                  Send requests through the proxy to see events
                </div>
              )}
            </div>
          </div>
        </div>

      </div>
    </div>
  )
}
