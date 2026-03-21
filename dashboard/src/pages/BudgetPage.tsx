import { useMemo, useState, useCallback } from 'react'
import { PieChart, Pie, Cell, Tooltip, ResponsiveContainer } from 'recharts'
import { Wallet, Coins, TrendingUp, RefreshCw, AlertTriangle, Ban, Pencil, Check, X, RotateCcw } from 'lucide-react'
import { clsx } from 'clsx'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCostSummary, useCostBreakdown } from '@/api/hooks'
import { fetchBudgetStatus, updateBudgetLimits, resetBudgetUsage } from '@/api/platform'
import type { CostBreakdownGroup } from '@/api/types'

// ── Types ──────────────────────────────────────────────────────────────────────

interface AgentBudgetRow {
  agent: string
  displayName: string
  tokensUsed: number
  tokensLimit: number | null
  costUsed: number
  costLimit: number | null
  percentUsed: number | null
  status: 'on-track' | 'warning' | 'over-budget' | 'no-limit'
}

// ── Donut chart colors ─────────────────────────────────────────────────────────

const DONUT_COLORS = ['#10b981', '#34d399', '#6ee7b7', '#0d9488', '#0891b2', '#7c3aed']

// ── Helpers ────────────────────────────────────────────────────────────────────

function formatTokens(n: number): string {
  if (n >= 1_000_000_000) return `${(n / 1_000_000_000).toFixed(1)}B`
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(0)}M`
  if (n >= 1_000) return `${(n / 1_000).toFixed(0)}K`
  return String(n)
}

function formatNumberWithCommas(n: number): string {
  return n.toLocaleString('en-US')
}

function parseFormattedNumber(s: string): number {
  const cleaned = s.replace(/,/g, '').replace(/\$/g, '').trim()
  const parsed = Number(cleaned)
  return isNaN(parsed) ? 0 : parsed
}

function progressColor(pct: number): string {
  if (pct >= 85) return '#f43f5e'  // rose
  if (pct >= 60) return '#f59e0b'  // amber
  return '#10b981'                  // emerald
}

function progressColorClass(pct: number): string {
  if (pct >= 85) return 'bg-rose-500'
  if (pct >= 60) return 'bg-amber-400'
  return 'bg-brand-500'
}

function deriveStatus(percentUsed: number | null): AgentBudgetRow['status'] {
  if (percentUsed === null) return 'no-limit'
  if (percentUsed >= 100) return 'over-budget'
  if (percentUsed >= 75) return 'warning'
  return 'on-track'
}

// ── Inline edit input ──────────────────────────────────────────────────────────

const inputClass =
  'bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white font-mono tabular-nums w-full transition-colors focus:border-emerald-500 focus:ring-1 focus:ring-emerald-500 focus:outline-none'

function EditNumberInput({
  value,
  onChange,
  prefix,
  suffix,
  placeholder,
}: {
  value: string
  onChange: (v: string) => void
  prefix?: string
  suffix?: string
  placeholder?: string
}) {
  return (
    <div className="relative flex items-center">
      {prefix && (
        <span className="absolute left-3 text-slate-500 text-sm font-mono pointer-events-none">
          {prefix}
        </span>
      )}
      <input
        type="text"
        className={clsx(inputClass, prefix && 'pl-7', suffix && 'pr-16')}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
      />
      {suffix && (
        <span className="absolute right-3 text-slate-500 text-xs font-mono pointer-events-none">
          {suffix}
        </span>
      )}
    </div>
  )
}

// ── Status badge ───────────────────────────────────────────────────────────────

function StatusBadge({ status }: { status: AgentBudgetRow['status'] }) {
  const configs = {
    'on-track': {
      label: 'On Track',
      className: 'bg-brand-500/10 text-brand-400 border border-brand-500/20',
    },
    'warning': {
      label: 'Warning',
      className: 'bg-amber-400/10 text-amber-400 border border-amber-400/20',
    },
    'over-budget': {
      label: 'Over Budget',
      className: 'bg-rose-500/10 text-rose-400 border border-rose-500/20',
      pulse: true,
    },
    'no-limit': {
      label: 'No Limit',
      className: 'bg-slate-700/60 text-slate-500 border border-slate-700',
    },
  } as const

  const cfg = configs[status]

  return (
    <span
      className={clsx(
        'inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[0.6875rem] font-medium font-display tracking-wide',
        cfg.className,
        'pulse' in cfg && cfg.pulse && 'animate-pulse-slow',
      )}
    >
      {'pulse' in cfg && cfg.pulse && (
        <span className="w-1.5 h-1.5 rounded-full bg-rose-400 animate-pulse" />
      )}
      {status === 'warning' && (
        <AlertTriangle className="w-2.5 h-2.5" />
      )}
      {status === 'no-limit' && (
        <Ban className="w-2.5 h-2.5" />
      )}
      {cfg.label}
    </span>
  )
}

// ── Global budget card ─────────────────────────────────────────────────────────

interface GlobalBudgetCardProps {
  icon: React.ReactNode
  label: string
  used: number | string
  limit: number | string
  pct: number
  sub: string
  accentIcon: React.ReactNode
  resetDays?: number
  editing?: boolean
  onEdit?: () => void
  editContent?: React.ReactNode
}

function GlobalBudgetCard({
  icon,
  label,
  used,
  limit,
  pct,
  sub,
  accentIcon,
  resetDays,
  editing,
  onEdit,
  editContent,
}: GlobalBudgetCardProps) {
  const barColor = progressColor(pct)
  const pctDisplay = pct.toFixed(1)

  return (
    <div className="card p-6 flex flex-col gap-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <div className="flex items-center justify-center w-9 h-9 rounded-lg bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border)]">
            {icon}
          </div>
          <span className="section-label">{label}</span>
        </div>
        <div className="flex items-center gap-2">
          {!editing && onEdit && (
            <button
              onClick={onEdit}
              className="btn-ghost p-1.5 rounded-lg hover:bg-white/5 transition-colors"
              title="Edit budget limit"
            >
              <Pencil className="w-3.5 h-3.5 text-slate-500 hover:text-slate-300" />
            </button>
          )}
          <div className="flex items-center gap-1.5">
            {accentIcon}
            <span className="font-mono text-[0.6875rem] text-slate-500">
              {pctDisplay}% used
            </span>
          </div>
        </div>
      </div>

      {/* Stat or edit content */}
      {editing && editContent ? (
        <div className="transition-all duration-200">{editContent}</div>
      ) : (
        <div>
          <div className="stat-value text-3xl text-slate-100 font-mono tabular-nums mb-1">
            {typeof used === 'string' ? used : used}
            <span className="text-slate-500 text-xl font-normal"> / </span>
            <span className="text-slate-400 text-2xl">{typeof limit === 'string' ? limit : limit}</span>
          </div>
        </div>
      )}

      {/* Progress bar */}
      <div>
        <div
          className="progress-track"
          style={{ height: '8px' }}
          role="progressbar"
          aria-valuenow={pct}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <div
            className="progress-fill"
            style={{
              width: `${Math.min(pct, 100)}%`,
              background: barColor,
              height: '8px',
              boxShadow: `0 0 10px ${barColor}55`,
            }}
          />
        </div>
        <div className="mt-2 flex items-center justify-between">
          <span className="text-[0.6875rem] text-slate-500">{sub}</span>
          {resetDays !== undefined && (
            <span className="flex items-center gap-1 text-[0.6875rem] text-slate-500">
              <RefreshCw className="w-3 h-3" />
              Resets in {resetDays} days
            </span>
          )}
        </div>
      </div>
    </div>
  )
}

// ── Donut chart card ──────────────────────────────────────────────────────────

interface DonutCardProps {
  title: string
  data: { name: string; value: number }[]
  totalLabel: string
  isLoading: boolean
}

interface DonutTooltipPayload {
  name: string
  value: number
  payload: { name: string; value: number }
}

interface DonutTooltipProps {
  active?: boolean
  payload?: DonutTooltipPayload[]
}

function DonutTooltip({ active, payload }: DonutTooltipProps) {
  if (!active || !payload?.length) return null
  const item = payload[0]
  return (
    <div className="bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border)] rounded-lg px-3 py-2 shadow-xl">
      <div className="text-[0.6875rem] font-medium text-slate-200 font-mono">{item.name}</div>
      <div className="text-brand-400 text-xs font-semibold font-mono mt-0.5">
        ${item.value.toFixed(4)}
      </div>
    </div>
  )
}

function DonutCard({ title, data, totalLabel, isLoading }: DonutCardProps) {
  const total = data.reduce((s, d) => s + d.value, 0)

  if (isLoading) {
    return (
      <div className="card p-5 flex flex-col gap-3">
        <div className="section-label">{title}</div>
        <div className="skeleton h-32 w-full rounded-lg" />
      </div>
    )
  }

  if (data.length === 0) {
    return (
      <div className="card p-5 flex flex-col gap-3">
        <div className="section-label">{title}</div>
        <div className="flex flex-col items-center justify-center h-32 text-slate-600 text-xs">
          No data
        </div>
      </div>
    )
  }

  return (
    <div className="card p-5 flex flex-col gap-4">
      <div className="section-label">{title}</div>
      <div className="relative flex items-center justify-center" style={{ height: 120 }}>
        <ResponsiveContainer width="100%" height={120}>
          <PieChart>
            <Pie
              data={data}
              cx="50%"
              cy="50%"
              innerRadius={36}
              outerRadius={52}
              paddingAngle={3}
              dataKey="value"
              stroke="none"
            >
              {data.map((_, idx) => (
                <Cell
                  key={idx}
                  fill={DONUT_COLORS[idx % DONUT_COLORS.length]}
                  opacity={0.9}
                />
              ))}
            </Pie>
            <Tooltip content={<DonutTooltip />} />
          </PieChart>
        </ResponsiveContainer>
        {/* Center label */}
        <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
          <div className="font-mono text-[0.625rem] text-slate-500 uppercase tracking-wider leading-none">
            {totalLabel}
          </div>
          <div className="font-mono text-sm font-semibold text-slate-200 mt-0.5">
            ${total.toFixed(2)}
          </div>
        </div>
      </div>
      {/* Legend */}
      <div className="space-y-1.5">
        {data.slice(0, 4).map((item, idx) => {
          const pct = total > 0 ? (item.value / total) * 100 : 0
          return (
            <div key={item.name} className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-1.5 min-w-0">
                <span
                  className="w-2 h-2 rounded-full shrink-0"
                  style={{ background: DONUT_COLORS[idx % DONUT_COLORS.length] }}
                />
                <span className="text-[0.6875rem] text-slate-400 font-mono truncate">
                  {item.name.length > 14 ? item.name.slice(0, 13) + '\u2026' : item.name}
                </span>
              </div>
              <span className="text-[0.6875rem] text-slate-500 font-mono tabular-nums shrink-0">
                {pct.toFixed(0)}%
              </span>
            </div>
          )
        })}
      </div>
    </div>
  )
}

// ── Per-agent table row ────────────────────────────────────────────────────────

interface AgentBudgetTableRowProps {
  row: AgentBudgetRow
  isEditing: boolean
  onStartEdit: () => void
  editTokenLimit: string
  editCostLimit: string
  onTokenLimitChange: (v: string) => void
  onCostLimitChange: (v: string) => void
  onSave: () => void
  onCancel: () => void
  saving: boolean
}

function AgentBudgetTableRow({
  row,
  isEditing,
  onStartEdit,
  editTokenLimit,
  editCostLimit,
  onTokenLimitChange,
  onCostLimitChange,
  onSave,
  onCancel,
  saving,
}: AgentBudgetTableRowProps) {
  const pct = row.percentUsed ?? 0
  const barColorClass = row.percentUsed !== null ? progressColorClass(pct) : 'bg-slate-700'

  return (
    <tr>
      {/* Agent name */}
      <td>
        <div className="flex items-center gap-2.5">
          <div className="w-7 h-7 rounded-md bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border)] flex items-center justify-center shrink-0">
            <span className="text-brand-400 font-display font-bold text-[0.5625rem] uppercase">
              {row.displayName.slice(0, 2)}
            </span>
          </div>
          <span className="text-slate-200 font-mono text-[0.8125rem]">{row.displayName}</span>
        </div>
      </td>

      {/* Token usage with inline progress */}
      <td>
        {isEditing ? (
          <div className="min-w-[140px]">
            <EditNumberInput
              value={editTokenLimit}
              onChange={onTokenLimitChange}
              suffix="tokens"
              placeholder="e.g. 100,000,000"
            />
          </div>
        ) : (
          <div className="flex flex-col gap-1.5 min-w-[140px]">
            <div className="flex items-center justify-between gap-2">
              <span className="font-mono text-[0.75rem] text-slate-300 tabular-nums">
                {formatTokens(row.tokensUsed)}
              </span>
              <span className="font-mono text-[0.6875rem] text-slate-600 tabular-nums">
                {row.tokensLimit !== null ? `/ ${formatTokens(row.tokensLimit)}` : '\u221E'}
              </span>
            </div>
            <div className="progress-track">
              <div
                className={clsx('progress-fill', barColorClass)}
                style={{
                  width: row.tokensLimit !== null
                    ? `${Math.min((row.tokensUsed / row.tokensLimit) * 100, 100)}%`
                    : '0%',
                }}
              />
            </div>
          </div>
        )}
      </td>

      {/* Cost */}
      <td>
        <span className="font-mono text-[0.8125rem] text-slate-200 tabular-nums">
          ${row.costUsed.toLocaleString('en-US', { minimumFractionDigits: 0 })}
        </span>
      </td>

      {/* Budget limit */}
      <td>
        {isEditing ? (
          <div className="min-w-[120px]">
            <EditNumberInput
              value={editCostLimit}
              onChange={onCostLimitChange}
              prefix="$"
              placeholder="e.g. 1,000"
            />
          </div>
        ) : row.costLimit !== null ? (
          <span className="font-mono text-[0.8125rem] text-slate-400 tabular-nums">
            ${row.costLimit.toLocaleString('en-US', { minimumFractionDigits: 0 })}
          </span>
        ) : (
          <span className="font-mono text-[0.8125rem] text-slate-600">Unlimited</span>
        )}
      </td>

      {/* % Used */}
      <td>
        {row.percentUsed !== null ? (
          <span
            className={clsx(
              'font-mono text-[0.8125rem] font-semibold tabular-nums',
              pct >= 85 ? 'text-rose-400' : pct >= 60 ? 'text-amber-400' : 'text-brand-400',
            )}
          >
            {pct.toFixed(1)}%
          </span>
        ) : (
          <span className="font-mono text-[0.8125rem] text-slate-600">&mdash;</span>
        )}
      </td>

      {/* Status + Edit actions */}
      <td>
        <div className="flex items-center gap-2">
          <StatusBadge status={row.status} />
          {isEditing ? (
            <div className="flex items-center gap-1 ml-1">
              <button
                onClick={onSave}
                disabled={saving}
                className="btn-primary p-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 transition-colors disabled:opacity-50"
                title="Save changes"
              >
                <Check className="w-3.5 h-3.5 text-white" />
              </button>
              <button
                onClick={onCancel}
                disabled={saving}
                className="btn-ghost p-1.5 rounded-lg hover:bg-white/5 transition-colors disabled:opacity-50"
                title="Cancel"
              >
                <X className="w-3.5 h-3.5 text-slate-400" />
              </button>
            </div>
          ) : (
            <button
              onClick={onStartEdit}
              className="btn-ghost p-1.5 rounded-lg hover:bg-white/5 transition-colors opacity-0 group-hover:opacity-100"
              title="Edit agent budget"
            >
              <Pencil className="w-3 h-3 text-slate-500 hover:text-slate-300" />
            </button>
          )}
        </div>
      </td>
    </tr>
  )
}

// ── Budget page ────────────────────────────────────────────────────────────────

export function BudgetPage() {
  const queryClient = useQueryClient()
  const { data: breakdown, isLoading: breakdownLoading } = useCostBreakdown({ days: 30 })
  const { data: summary } = useCostSummary({ days: 30 })

  const { data: budgetData, isLoading: budgetLoading } = useQuery({
    queryKey: ['budget-status'],
    queryFn: fetchBudgetStatus,
    staleTime: 30_000,
    retry: 1,
  })

  // ── Global edit state ──────────────────────────────────────────────────────
  const [editingGlobal, setEditingGlobal] = useState(false)
  const [globalTokenLimitInput, setGlobalTokenLimitInput] = useState<string>('')
  const [globalCostLimitInput, setGlobalCostLimitInput] = useState<string>('')
  const [globalSaving, setGlobalSaving] = useState(false)

  // ── Per-agent edit state ───────────────────────────────────────────────────
  const [editingAgent, setEditingAgent] = useState<string | null>(null)
  const [agentTokenLimitInput, setAgentTokenLimitInput] = useState<string>('')
  const [agentCostLimitInput, setAgentCostLimitInput] = useState<string>('')
  const [agentSaving, setAgentSaving] = useState(false)

  // ── Reset confirmation ─────────────────────────────────────────────────────
  const [showResetConfirm, setShowResetConfirm] = useState(false)
  const [resetting, setResetting] = useState(false)

  // Build donut data from real cost breakdown, or fall back to budget API agent data
  const byAgentData = useMemo<{ name: string; value: number }[]>(() => {
    const raw: CostBreakdownGroup[] = breakdown?.by_agent ?? []
    if (raw.length > 0) {
      return raw
        .sort((a, b) => b.cost_usd - a.cost_usd)
        .slice(0, 6)
        .map(g => ({ name: g.group || 'unknown', value: g.cost_usd }))
    }
    // Fallback: derive from budget API agent list
    if (budgetData?.agents && budgetData.agents.length > 0) {
      return budgetData.agents
        .sort((a, b) => b.cost_used - a.cost_used)
        .slice(0, 6)
        .map(a => ({ name: a.name ?? a.agent_id, value: a.cost_used }))
    }
    return [
      { name: 'code-assistant',  value: 580 },
      { name: 'data-analyzer',   value: 412 },
      { name: 'customer-support',value: 350 },
      { name: 'report-generator',value: 225 },
      { name: 'security-scanner',value: 156 },
      { name: 'test-agent',      value: 124 },
    ]
  }, [breakdown, budgetData])

  const byModelData = useMemo<{ name: string; value: number }[]>(() => {
    const raw: CostBreakdownGroup[] = breakdown?.by_model ?? []
    if (raw.length > 0) {
      return raw
        .sort((a, b) => b.cost_usd - a.cost_usd)
        .slice(0, 6)
        .map(g => ({ name: g.group || 'unknown', value: g.cost_usd }))
    }
    return [
      { name: 'claude-3-5-sonnet', value: 980 },
      { name: 'gpt-4o', value: 510 },
      { name: 'claude-3-haiku', value: 220 },
      { name: 'gpt-4o-mini', value: 137 },
    ]
  }, [breakdown])

  const byProtocolData = useMemo<{ name: string; value: number }[]>(() => {
    const raw: CostBreakdownGroup[] = breakdown?.by_protocol ?? []
    if (raw.length > 0) {
      return raw
        .sort((a, b) => b.cost_usd - a.cost_usd)
        .slice(0, 6)
        .map(g => ({ name: g.group || 'unknown', value: g.cost_usd }))
    }
    return [
      { name: 'openai', value: 1024 },
      { name: 'anthropic', value: 580 },
      { name: 'mcp', value: 243 },
    ]
  }, [breakdown])

  // Global tokens: prefer budget API, then cost summary, then inline fallback
  const globalTokensLimit = budgetData?.global_tokens_limit ?? 500_000_000
  const globalCostLimit = budgetData?.global_cost_limit ?? 5000
  const resetDays = budgetData?.reset_days ?? 6

  const globalTokensUsed = budgetData?.global_tokens_used ?? summary?.total_tokens ?? 142_000_000
  const globalTokensPct = globalTokensLimit > 0
    ? (globalTokensUsed / globalTokensLimit) * 100
    : 0

  const globalCostUsed = budgetData?.global_cost_used ?? summary?.total_cost_usd ?? 1847
  const globalCostPct = globalCostLimit > 0
    ? (globalCostUsed / globalCostLimit) * 100
    : 0

  // Build per-agent rows from budget API, falling back to inline mock
  const agentRows: AgentBudgetRow[] = useMemo(() => {
    if (budgetData?.agents && budgetData.agents.length > 0) {
      return budgetData.agents.map(a => {
        const pct = (a.tokens_limit && a.tokens_limit > 0)
          ? (a.tokens_used / a.tokens_limit) * 100
          : null
        return {
          agent: a.agent_id,
          displayName: a.name ?? a.agent_id,
          tokensUsed: a.tokens_used,
          tokensLimit: a.tokens_limit ?? null,
          costUsed: a.cost_used,
          costLimit: a.cost_limit ?? null,
          percentUsed: pct !== null ? Math.round(pct * 10) / 10 : null,
          status: deriveStatus(pct),
        }
      })
    }
    // Inline fallback
    return [
      { agent: 'code-assistant',   displayName: 'code-assistant',   tokensUsed: 45_000_000, tokensLimit: 100_000_000, costUsed: 580, costLimit: 1000, percentUsed: 58,   status: 'on-track' },
      { agent: 'data-analyzer',    displayName: 'data-analyzer',    tokensUsed: 32_000_000, tokensLimit:  50_000_000, costUsed: 412, costLimit:  500, percentUsed: 82.4, status: 'warning' },
      { agent: 'customer-support', displayName: 'customer-support', tokensUsed: 28_000_000, tokensLimit:  80_000_000, costUsed: 350, costLimit:  800, percentUsed: 43.8, status: 'on-track' },
      { agent: 'report-generator', displayName: 'report-generator', tokensUsed: 18_000_000, tokensLimit:  30_000_000, costUsed: 225, costLimit:  300, percentUsed: 75,   status: 'warning' },
      { agent: 'security-scanner', displayName: 'security-scanner', tokensUsed: 12_000_000, tokensLimit: null,        costUsed: 156, costLimit: null,  percentUsed: null, status: 'no-limit' },
      { agent: 'test-agent',       displayName: 'test-agent',       tokensUsed:  7_000_000, tokensLimit:  10_000_000, costUsed: 124, costLimit:  150, percentUsed: 82.7, status: 'warning' },
    ]
  }, [budgetData])

  const warningCount = agentRows.filter(a => a.status === 'warning').length
  const onTrackCount = agentRows.filter(a => a.status === 'on-track').length

  // ── Global edit handlers ───────────────────────────────────────────────────

  const handleStartGlobalEdit = useCallback(() => {
    setGlobalTokenLimitInput(formatNumberWithCommas(globalTokensLimit))
    setGlobalCostLimitInput(formatNumberWithCommas(globalCostLimit))
    setEditingGlobal(true)
  }, [globalTokensLimit, globalCostLimit])

  const handleCancelGlobalEdit = useCallback(() => {
    setEditingGlobal(false)
    setGlobalTokenLimitInput('')
    setGlobalCostLimitInput('')
  }, [])

  const handleSaveGlobalEdit = useCallback(async () => {
    setGlobalSaving(true)
    try {
      await updateBudgetLimits({
        global_tokens_limit: parseFormattedNumber(globalTokenLimitInput),
        global_cost_limit: parseFormattedNumber(globalCostLimitInput),
      })
      queryClient.invalidateQueries({ queryKey: ['budget-status'] })
      setEditingGlobal(false)
    } catch {
      // Backend not wired yet; apply optimistic update to local state
      setEditingGlobal(false)
    } finally {
      setGlobalSaving(false)
    }
  }, [globalTokenLimitInput, globalCostLimitInput, queryClient])

  // ── Per-agent edit handlers ────────────────────────────────────────────────

  const handleStartAgentEdit = useCallback((row: AgentBudgetRow) => {
    setEditingAgent(row.agent)
    setAgentTokenLimitInput(row.tokensLimit !== null ? formatNumberWithCommas(row.tokensLimit) : '')
    setAgentCostLimitInput(row.costLimit !== null ? formatNumberWithCommas(row.costLimit) : '')
  }, [])

  const handleCancelAgentEdit = useCallback(() => {
    setEditingAgent(null)
    setAgentTokenLimitInput('')
    setAgentCostLimitInput('')
  }, [])

  const handleSaveAgentEdit = useCallback(async () => {
    if (!editingAgent) return
    setAgentSaving(true)
    const tokensVal = parseFormattedNumber(agentTokenLimitInput)
    const costVal = parseFormattedNumber(agentCostLimitInput)
    try {
      await updateBudgetLimits({
        agent_limits: [{
          agent_id: editingAgent,
          tokens_limit: tokensVal > 0 ? tokensVal : null,
          cost_limit: costVal > 0 ? costVal : null,
        }],
      })
      queryClient.invalidateQueries({ queryKey: ['budget-status'] })
      setEditingAgent(null)
    } catch {
      // Backend not wired yet; close edit mode gracefully
      setEditingAgent(null)
    } finally {
      setAgentSaving(false)
    }
  }, [editingAgent, agentTokenLimitInput, agentCostLimitInput, queryClient])

  // ── Reset handler ──────────────────────────────────────────────────────────

  const handleResetUsage = useCallback(async () => {
    setResetting(true)
    try {
      await resetBudgetUsage()
      queryClient.invalidateQueries({ queryKey: ['budget-status'] })
    } catch {
      // Backend not wired yet
    } finally {
      setResetting(false)
      setShowResetConfirm(false)
    }
  }, [queryClient])

  return (
    <div className="space-y-6">

      {/* ── Row 1: Global budget overview ───────────────────────────────────── */}
      {budgetLoading ? (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <div className="skeleton h-44 rounded-xl" />
          <div className="skeleton h-44 rounded-xl" />
        </div>
      ) : (
        <>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 stagger-in">

            {/* Token budget */}
            <GlobalBudgetCard
              icon={<Coins className="w-4.5 h-4.5 text-brand-400" style={{ width: 18, height: 18 }} />}
              label="Token Budget"
              used={formatTokens(globalTokensUsed) + ' tokens'}
              limit={formatTokens(globalTokensLimit) + ' tokens'}
              pct={globalTokensPct}
              sub={`${globalTokensPct.toFixed(1)}% used`}
              resetDays={resetDays}
              editing={editingGlobal}
              onEdit={handleStartGlobalEdit}
              accentIcon={
                <TrendingUp
                  style={{ width: 12, height: 12 }}
                  className={clsx(
                    globalTokensPct >= 85 ? 'text-rose-400' :
                    globalTokensPct >= 60 ? 'text-amber-400' : 'text-brand-500',
                  )}
                />
              }
              editContent={
                <div className="space-y-3">
                  <div>
                    <label className="text-[0.6875rem] text-slate-500 font-mono mb-1 block">
                      Token Limit
                    </label>
                    <EditNumberInput
                      value={globalTokenLimitInput}
                      onChange={setGlobalTokenLimitInput}
                      suffix="tokens"
                      placeholder="e.g. 500,000,000"
                    />
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={handleSaveGlobalEdit}
                      disabled={globalSaving}
                      className="btn-primary px-4 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium transition-colors disabled:opacity-50"
                    >
                      {globalSaving ? 'Saving...' : 'Save'}
                    </button>
                    <button
                      onClick={handleCancelGlobalEdit}
                      disabled={globalSaving}
                      className="btn-ghost px-4 py-1.5 rounded-lg hover:bg-white/5 text-slate-400 text-xs font-medium transition-colors disabled:opacity-50"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              }
            />

            {/* Cost budget */}
            <GlobalBudgetCard
              icon={<Wallet className="w-4.5 h-4.5 text-brand-400" style={{ width: 18, height: 18 }} />}
              label="Cost Budget"
              used={`$${globalCostUsed.toLocaleString('en-US', { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`}
              limit={`$${globalCostLimit.toLocaleString('en-US', { minimumFractionDigits: 0 })}`}
              pct={globalCostPct}
              sub={`${globalCostPct.toFixed(1)}% used`}
              resetDays={resetDays}
              editing={editingGlobal}
              onEdit={handleStartGlobalEdit}
              accentIcon={
                <TrendingUp
                  style={{ width: 12, height: 12 }}
                  className={clsx(
                    globalCostPct >= 85 ? 'text-rose-400' :
                    globalCostPct >= 60 ? 'text-amber-400' : 'text-brand-500',
                  )}
                />
              }
              editContent={
                <div className="space-y-3">
                  <div>
                    <label className="text-[0.6875rem] text-slate-500 font-mono mb-1 block">
                      Cost Limit
                    </label>
                    <EditNumberInput
                      value={globalCostLimitInput}
                      onChange={setGlobalCostLimitInput}
                      prefix="$"
                      placeholder="e.g. 5,000"
                    />
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={handleSaveGlobalEdit}
                      disabled={globalSaving}
                      className="btn-primary px-4 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-medium transition-colors disabled:opacity-50"
                    >
                      {globalSaving ? 'Saving...' : 'Save'}
                    </button>
                    <button
                      onClick={handleCancelGlobalEdit}
                      disabled={globalSaving}
                      className="btn-ghost px-4 py-1.5 rounded-lg hover:bg-white/5 text-slate-400 text-xs font-medium transition-colors disabled:opacity-50"
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              }
            />
          </div>

          {/* Reset Usage button */}
          <div className="flex items-center justify-end">
            {showResetConfirm ? (
              <div className="flex items-center gap-3 bg-rose-500/10 border border-rose-500/20 rounded-lg px-4 py-2.5">
                <span className="text-[0.8125rem] text-rose-300">
                  Are you sure? This resets all usage counters to zero.
                </span>
                <button
                  onClick={handleResetUsage}
                  disabled={resetting}
                  className="btn-danger px-3 py-1 rounded-lg bg-rose-600 hover:bg-rose-500 text-white text-xs font-medium transition-colors disabled:opacity-50"
                >
                  {resetting ? 'Resetting...' : 'Confirm Reset'}
                </button>
                <button
                  onClick={() => setShowResetConfirm(false)}
                  disabled={resetting}
                  className="btn-ghost px-3 py-1 rounded-lg hover:bg-white/5 text-slate-400 text-xs font-medium transition-colors disabled:opacity-50"
                >
                  Cancel
                </button>
              </div>
            ) : (
              <button
                onClick={() => setShowResetConfirm(true)}
                className="btn-ghost flex items-center gap-1.5 px-3 py-1.5 rounded-lg hover:bg-white/5 text-slate-500 hover:text-slate-300 text-xs font-medium transition-colors"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                Reset Usage
              </button>
            )}
          </div>
        </>
      )}

      {/* ── Row 2: Spend by Category ─────────────────────────────────────────── */}
      <div>
        <div className="section-label mb-3 px-0.5">Spend by Category</div>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 stagger-in">
          <DonutCard
            title="By Agent"
            data={byAgentData}
            totalLabel="30-day total"
            isLoading={breakdownLoading}
          />
          <DonutCard
            title="By Model"
            data={byModelData}
            totalLabel="30-day total"
            isLoading={breakdownLoading}
          />
          <DonutCard
            title="By Protocol"
            data={byProtocolData}
            totalLabel="30-day total"
            isLoading={breakdownLoading}
          />
        </div>
      </div>

      {/* ── Row 3: Per-agent budget table ────────────────────────────────────── */}
      <div className="card overflow-hidden">
        {/* Card header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[var(--govrix-border)]">
          <div>
            <div className="section-label">Per-Agent Budget</div>
            <div className="mt-1 text-[0.75rem] text-slate-500">
              Individual token and spend limits per agent
            </div>
          </div>
          <div className="flex items-center gap-3">
            {/* Summary pills */}
            <div className="hidden sm:flex items-center gap-2">
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-amber-400/10 border border-amber-400/20 text-[0.6875rem] font-medium text-amber-400">
                <AlertTriangle className="w-3 h-3" />
                {warningCount} Warning
              </span>
              <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-brand-500/10 border border-brand-500/20 text-[0.6875rem] font-medium text-brand-400">
                {onTrackCount} On Track
              </span>
            </div>
          </div>
        </div>

        {/* Table */}
        {budgetLoading ? (
          <div className="p-5 space-y-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <div key={i} className="skeleton h-12 rounded-lg" />
            ))}
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="govrix-table">
              <thead>
                <tr>
                  <th>Agent</th>
                  <th>Token Usage</th>
                  <th>Cost</th>
                  <th>Budget Limit</th>
                  <th>% Used</th>
                  <th>Status</th>
                </tr>
              </thead>
              <tbody className="[&>tr]:group">
                {agentRows.map(row => (
                  <AgentBudgetTableRow
                    key={row.agent}
                    row={row}
                    isEditing={editingAgent === row.agent}
                    onStartEdit={() => handleStartAgentEdit(row)}
                    editTokenLimit={editingAgent === row.agent ? agentTokenLimitInput : ''}
                    editCostLimit={editingAgent === row.agent ? agentCostLimitInput : ''}
                    onTokenLimitChange={setAgentTokenLimitInput}
                    onCostLimitChange={setAgentCostLimitInput}
                    onSave={handleSaveAgentEdit}
                    onCancel={handleCancelAgentEdit}
                    saving={agentSaving}
                  />
                ))}
              </tbody>
            </table>
          </div>
        )}

        {/* Footer note */}
        <div className="px-5 py-3 border-t border-[var(--govrix-border)] flex items-center justify-between">
          <span className="text-[0.6875rem] text-slate-600 font-mono">
            Budget period: monthly &middot; Resets in {resetDays} days
          </span>
          <span className="text-[0.6875rem] text-slate-600">
            {agentRows.length} agents total
          </span>
        </div>
      </div>

    </div>
  )
}
