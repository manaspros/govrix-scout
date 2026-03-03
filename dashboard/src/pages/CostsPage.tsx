import React, { useState } from 'react'
import {
  BarChart, Bar, PieChart, Pie, Cell,
  XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid, Legend,
} from 'recharts'
import { DollarSign, RefreshCw, TrendingUp, Layers, Activity } from 'lucide-react'
import { useCostSummary, useCostBreakdown } from '../api/hooks'
import type { ElementType } from 'react'

const COLORS = ['#6366f1', '#8b5cf6', '#0ea5e9', '#10b981', '#f59e0b', '#f43f5e', '#64748b']

const fmtNum = (n: number | undefined | null): string =>
  typeof n === 'number' ? n.toLocaleString() : '0'

const fmtUsd = (n: number | undefined | null): string =>
  typeof n === 'number' ? `$${n.toFixed(4)}` : '$0.00'

interface KPIProps {
  label: string
  value: string
  icon: ElementType
  sub?: string
}

const KPI = ({ label, value, icon: Icon, sub }: KPIProps) => (
  <div className="stat-card">
    <div className="flex items-center justify-between mb-2">
      <span className="text-[10px] uppercase tracking-widest text-slate-400 font-bold">{label}</span>
      <Icon className="w-4 h-4 text-primary" />
    </div>
    <div className="text-2xl font-black text-slate-900 metric-font">{value}</div>
    {sub && <p className="text-[10px] text-slate-400 mt-1">{sub}</p>}
  </div>
)

type GroupBy = 'model' | 'agent' | 'provider'

export default function CostsPage() {
  const [groupBy, setGroupBy] = useState<GroupBy>('model')

  const { data: summary, refetch } = useCostSummary()
  const { data: breakdown } = useCostBreakdown()

  const breakdownRows = groupBy === 'model'
    ? (breakdown?.by_model ?? [])
    : groupBy === 'agent'
    ? (breakdown?.by_agent ?? [])
    : (breakdown?.by_provider ?? [])

  // Bar chart data for cost by selected group (replacing timeseries AreaChart)
  const barData = breakdownRows.slice(0, 7).map(r => ({
    name: r.label || 'unknown',
    cost: r.cost_usd || 0,
    requests: r.requests || 0,
  }))

  // Donut pie data
  const pieData = breakdownRows.slice(0, 7).map(r => ({
    name: r.label || 'unknown',
    value: r.cost_usd || 0,
  }))

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-[1400px] mx-auto space-y-6">

        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-bold text-slate-900">Cost Analytics</h2>
            <p className="text-xs text-slate-400">Track AI spend across models and agents</p>
          </div>
          <button
            onClick={() => refetch()}
            className="btn-secondary flex items-center gap-1.5 text-xs"
          >
            <RefreshCw className="w-3.5 h-3.5" /> Refresh
          </button>
        </div>

        {/* KPI Row */}
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
          <KPI
            label="Total Cost"
            value={fmtUsd(summary?.total_cost_usd)}
            icon={DollarSign}
            sub="All time"
          />
          <KPI
            label="Total Requests"
            value={fmtNum(summary?.total_requests)}
            icon={TrendingUp}
            sub="All time"
          />
          <KPI
            label="Tokens Used"
            value={fmtNum(
              (summary?.total_input_tokens ?? 0) + (summary?.total_output_tokens ?? 0)
            )}
            icon={Layers}
            sub={`${fmtNum(summary?.total_input_tokens)} in / ${fmtNum(summary?.total_output_tokens)} out`}
          />
          <KPI
            label="Avg Cost / Req"
            value={fmtUsd(summary?.avg_cost_per_request)}
            icon={Activity}
            sub="Per request average"
          />
        </div>

        {/* Cost Bar Chart by selected group (no timeseries API in Scout) */}
        <div className="bg-white border border-slate-200 rounded-xl p-5">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-sm font-bold text-slate-700">
              Cost by {groupBy.charAt(0).toUpperCase() + groupBy.slice(1)}
            </h3>
            <div className="flex gap-1">
              {(['model', 'agent', 'provider'] as GroupBy[]).map(g => (
                <button
                  key={g}
                  onClick={() => setGroupBy(g)}
                  className={`text-[10px] font-bold px-2.5 py-1 rounded-md transition-colors ${
                    groupBy === g ? 'bg-primary text-white' : 'text-slate-400 hover:bg-slate-100'
                  }`}
                >
                  {g.toUpperCase()}
                </button>
              ))}
            </div>
          </div>
          {barData.length > 0 ? (
            <ResponsiveContainer width="100%" height={240}>
              <BarChart data={barData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
                <XAxis
                  dataKey="name"
                  tick={{ fontSize: 10, fill: '#94a3b8' }}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis
                  yAxisId="cost"
                  tick={{ fontSize: 10, fill: '#94a3b8' }}
                  axisLine={false}
                  tickLine={false}
                  width={55}
                  tickFormatter={(v: number) => `$${v.toFixed(3)}`}
                />
                <YAxis
                  yAxisId="reqs"
                  orientation="right"
                  tick={{ fontSize: 10, fill: '#94a3b8' }}
                  axisLine={false}
                  tickLine={false}
                  width={40}
                />
                <Tooltip
                  contentStyle={{ fontSize: 11, borderRadius: 8, border: '1px solid #e2e8f0' }}
                  formatter={(v: number, name: string) =>
                    name === 'Cost ($)' ? [`$${v.toFixed(5)}`, name] : [v.toLocaleString(), name]
                  }
                />
                <Legend iconType="circle" iconSize={6} wrapperStyle={{ fontSize: 10 }} />
                <Bar
                  yAxisId="cost"
                  dataKey="cost"
                  fill="#6366f1"
                  radius={[4, 4, 0, 0]}
                  name="Cost ($)"
                />
                <Bar
                  yAxisId="reqs"
                  dataKey="requests"
                  fill="#0ea5e9"
                  radius={[4, 4, 0, 0]}
                  name="Requests"
                />
              </BarChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-[240px] flex items-center justify-center text-slate-400 text-sm">
              No cost data available
            </div>
          )}
        </div>

        {/* Breakdown Row */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {/* Cost Breakdown Table */}
          <div className="bg-white border border-slate-200 rounded-xl p-5">
            <div className="flex items-center justify-between mb-4">
              <h3 className="text-sm font-bold text-slate-700">Cost Breakdown</h3>
              <div className="flex gap-1">
                {(['model', 'agent', 'provider'] as GroupBy[]).map(g => (
                  <button
                    key={g}
                    onClick={() => setGroupBy(g)}
                    className={`text-[10px] font-bold px-2.5 py-1 rounded-md transition-colors ${
                      groupBy === g ? 'bg-primary text-white' : 'text-slate-400 hover:bg-slate-100'
                    }`}
                  >
                    {g.toUpperCase()}
                  </button>
                ))}
              </div>
            </div>
            <div className="space-y-2 max-h-[260px] overflow-y-auto">
              {breakdownRows.map((row, i) => (
                <div
                  key={i}
                  className="flex items-center justify-between py-2 border-b border-slate-50 last:border-0"
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <span
                      className="w-2.5 h-2.5 rounded-full flex-shrink-0"
                      style={{ background: COLORS[i % COLORS.length] }}
                    />
                    <span className="text-xs text-slate-700 font-medium truncate max-w-[150px]">
                      {row.label || 'unknown'}
                    </span>
                  </div>
                  <div className="flex items-center gap-4 flex-shrink-0">
                    <span className="text-[10px] metric-font text-slate-400">
                      {fmtNum(row.requests)} req
                    </span>
                    <span className="text-xs metric-font font-semibold text-slate-700">
                      {fmtUsd(row.cost_usd)}
                    </span>
                  </div>
                </div>
              ))}
              {breakdownRows.length === 0 && (
                <div className="text-center py-8 text-slate-400 text-sm">
                  No cost data available
                </div>
              )}
            </div>
          </div>

          {/* Cost Distribution Donut */}
          <div className="bg-white border border-slate-200 rounded-xl p-5">
            <h3 className="text-sm font-bold text-slate-700 mb-4">
              Distribution by {groupBy}
            </h3>
            {pieData.length > 0 ? (
              <>
                <ResponsiveContainer width="100%" height={220}>
                  <PieChart>
                    <Pie
                      data={pieData}
                      cx="50%"
                      cy="50%"
                      innerRadius={55}
                      outerRadius={85}
                      paddingAngle={2}
                      dataKey="value"
                      nameKey="name"
                    >
                      {pieData.map((_, i) => (
                        <Cell key={i} fill={COLORS[i % COLORS.length]} />
                      ))}
                    </Pie>
                    <Tooltip
                      contentStyle={{ fontSize: 11, borderRadius: 8 }}
                      formatter={(v: number) => [`$${v.toFixed(5)}`, 'Cost']}
                    />
                  </PieChart>
                </ResponsiveContainer>
                <div className="flex flex-wrap gap-3 mt-3 justify-center">
                  {pieData.map((d, i) => (
                    <span key={i} className="text-[10px] flex items-center gap-1.5">
                      <span
                        className="w-2 h-2 rounded-full"
                        style={{ background: COLORS[i % COLORS.length] }}
                      />
                      {d.name}
                    </span>
                  ))}
                </div>
              </>
            ) : (
              <div className="h-[260px] flex items-center justify-center text-slate-400 text-sm">
                <DollarSign className="w-8 h-8 text-slate-300 mr-2" />
                No cost data yet
              </div>
            )}
          </div>
        </div>

      </div>
    </div>
  )
}
