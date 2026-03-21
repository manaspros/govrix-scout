import { useState, useMemo } from 'react'
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from 'recharts'
import { format, parseISO } from 'date-fns'
import { DollarSign, Activity, Zap } from 'lucide-react'
import { useCostSummary, useCostBreakdown } from '@/api/hooks'
import { TimeRangePicker, timeRangeToDays } from '@/components/common/TimeRangePicker'
import type { TimeRange } from '@/components/common/TimeRangePicker'
import { CardSkeleton, ChartSkeleton } from '@/components/common/LoadingState'
import type { CostBreakdownGroup } from '@/api/types'

// ── Stat card ─────────────────────────────────────────────────────────────────

interface StatCardProps {
  label: string
  value: string
  sub?: string
  icon: React.ReactNode
}

function StatCard({ label, value, sub, icon }: StatCardProps) {
  return (
    <div className="bg-slate-800 rounded-xl p-5 border border-slate-700/60 flex items-start gap-4">
      <div className="flex items-center justify-center w-10 h-10 rounded-lg bg-slate-700/60 shrink-0">
        {icon}
      </div>
      <div>
        <div className="text-xs font-medium text-slate-400 uppercase tracking-wider mb-1">{label}</div>
        <div className="text-2xl font-bold text-white tabular-nums">{value}</div>
        {sub && <div className="text-xs text-slate-500 mt-0.5">{sub}</div>}
      </div>
    </div>
  )
}

// ── Breakdown table ───────────────────────────────────────────────────────────

interface BreakdownTableProps {
  data: CostBreakdownGroup[]
  groupLabel: string
  isLoading: boolean
}

function BreakdownTable({ data, groupLabel, isLoading }: BreakdownTableProps) {
  const sorted = useMemo(
    () => [...(data ?? [])].sort((a, b) => b.cost_usd - a.cost_usd),
    [data],
  )
  const total = sorted.reduce((s, r) => s + r.cost_usd, 0)

  if (isLoading) return <div className="animate-pulse h-40 bg-slate-700/30 rounded-xl" />

  return (
    <div className="overflow-x-auto rounded-xl border border-slate-700">
      <table className="w-full text-sm">
        <thead className="bg-slate-800/80">
          <tr>
            {[groupLabel, 'Requests', 'Tokens', 'Cost USD', '% of Total'].map(h => (
              <th key={h} className="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y divide-slate-700/40">
          {sorted.length === 0 ? (
            <tr>
              <td colSpan={5} className="px-4 py-8 text-center text-slate-500 text-xs">
                No data for this period
              </td>
            </tr>
          ) : (
            sorted.map((row, i) => (
              <tr key={i} className="hover:bg-slate-700/20 transition-colors">
                <td className="px-4 py-3 text-slate-200 font-mono text-xs max-w-[200px] truncate">
                  {row.group || '—'}
                </td>
                <td className="px-4 py-3 text-slate-300 tabular-nums text-right">
                  {row.requests.toLocaleString()}
                </td>
                <td className="px-4 py-3 text-slate-300 tabular-nums text-right">
                  {row.tokens.toLocaleString()}
                </td>
                <td className="px-4 py-3 text-emerald-400 tabular-nums text-right font-semibold">
                  ${row.cost_usd.toFixed(6)}
                </td>
                <td className="px-4 py-3 text-slate-400 tabular-nums text-right">
                  {total > 0 ? `${((row.cost_usd / total) * 100).toFixed(1)}%` : '—'}
                </td>
              </tr>
            ))
          )}
        </tbody>
      </table>
    </div>
  )
}

// ── Tooltip ───────────────────────────────────────────────────────────────────

interface TooltipPayload {
  name: string
  value: number
  color: string
}

interface CustomTooltipProps {
  active?: boolean
  payload?: TooltipPayload[]
  label?: string
}

function CostTooltip({ active, payload, label }: CustomTooltipProps) {
  if (!active || !payload?.length) return null
  return (
    <div className="bg-slate-800 border border-slate-600 rounded-lg px-3 py-2 shadow-xl text-xs">
      <div className="text-slate-400 mb-1">{label}</div>
      {payload.map(p => (
        <div key={p.name} className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full" style={{ background: p.color }} />
          <span className="text-slate-300">{p.name}:</span>
          <span className="text-white font-semibold">
            {p.name === 'cost' ? `$${p.value.toFixed(6)}` : p.value.toLocaleString()}
          </span>
        </div>
      ))}
    </div>
  )
}

// ── Costs page ────────────────────────────────────────────────────────────────

type BreakdownTab = 'by_agent' | 'by_model' | 'by_protocol'

const TABS: { key: BreakdownTab; label: string }[] = [
  { key: 'by_agent',    label: 'By Agent' },
  { key: 'by_model',   label: 'By Model' },
  { key: 'by_protocol', label: 'By Protocol' },
]

export function CostsPage() {
  const [timeRange, setTimeRange] = useState<TimeRange>('7d')
  const [activeTab, setActiveTab] = useState<BreakdownTab>('by_agent')

  const days = timeRangeToDays(timeRange)
  const { data: summary, isLoading: summaryLoading } = useCostSummary({ days })
  const { data: breakdown, isLoading: breakdownLoading } = useCostBreakdown({ days })

  const dailyChart = useMemo(() => {
    if (!breakdown?.daily) return []
    return breakdown.daily.map(d => ({
      date: format(parseISO(d.timestamp), 'MMM d'),
      cost: d.cost_usd,
      requests: d.requests,
      tokens: d.tokens,
    }))
  }, [breakdown])

  const tabData: CostBreakdownGroup[] = breakdown?.[activeTab] ?? []

  return (
    <div className="space-y-6">
      {/* Header row */}
      <div className="flex items-center justify-between">
        <div />
        <TimeRangePicker value={timeRange} onChange={setTimeRange} />
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        {summaryLoading ? (
          <><CardSkeleton /><CardSkeleton /><CardSkeleton /></>
        ) : (
          <>
            <StatCard
              label="Total Cost"
              value={`$${(summary?.total_cost_usd ?? 0).toFixed(6)}`}
              sub={`Last ${days} day${days !== 1 ? 's' : ''}`}
              icon={<DollarSign className="w-5 h-5 text-emerald-400" />}
            />
            <StatCard
              label="Total Requests"
              value={(summary?.total_requests ?? 0).toLocaleString()}
              sub="Intercepted calls"
              icon={<Activity className="w-5 h-5 text-brand-400" />}
            />
            <StatCard
              label="Total Tokens"
              value={(summary?.total_tokens ?? 0).toLocaleString()}
              sub="Input + output"
              icon={<Zap className="w-5 h-5 text-violet-400" />}
            />
          </>
        )}
      </div>

      {/* Area chart */}
      <div className="bg-slate-800 rounded-xl border border-slate-700/60 p-5">
        <h2 className="text-sm font-semibold text-slate-200 mb-4">Daily Cost Trend</h2>
        {breakdownLoading ? (
          <ChartSkeleton height={200} />
        ) : (
          <ResponsiveContainer width="100%" height={200}>
            <AreaChart data={dailyChart} margin={{ top: 2, right: 4, left: -10, bottom: 0 }}>
              <defs>
                <linearGradient id="costGrad" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#10b981" stopOpacity={0.35} />
                  <stop offset="95%" stopColor="#10b981" stopOpacity={0} />
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" stroke="#334155" />
              <XAxis dataKey="date" tick={{ fontSize: 11, fill: '#94a3b8' }} />
              <YAxis
                tick={{ fontSize: 11, fill: '#94a3b8' }}
                tickFormatter={v => `$${Number(v).toFixed(4)}`}
              />
              <Tooltip content={<CostTooltip />} />
              <Area
                type="monotone"
                dataKey="cost"
                stroke="#10b981"
                strokeWidth={2}
                fill="url(#costGrad)"
                dot={false}
              />
            </AreaChart>
          </ResponsiveContainer>
        )}
      </div>

      {/* Breakdown table */}
      <div className="bg-slate-800 rounded-xl border border-slate-700/60">
        <div className="flex items-center gap-1 px-4 py-3 border-b border-slate-700/60">
          {TABS.map(tab => (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={`px-4 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                activeTab === tab.key
                  ? 'bg-brand-600/20 text-brand-400 ring-1 ring-brand-600/30'
                  : 'text-slate-400 hover:text-slate-200 hover:bg-slate-700'
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <div className="p-4">
          <BreakdownTable
            data={tabData}
            groupLabel={activeTab === 'by_agent' ? 'Agent' : activeTab === 'by_model' ? 'Model' : 'Protocol'}
            isLoading={breakdownLoading}
          />
        </div>
      </div>
    </div>
  )
}
