import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts'
import { ScanEye, ShieldCheck, Layers } from 'lucide-react'
import { clsx } from 'clsx'
import { useQuery } from '@tanstack/react-query'
import { fetchPiiActivity } from '@/api/platform'
import { EnterpriseFeatureCard, isNotFoundError } from '@/components/common/EnterpriseFeatureCard'

// ── Types ──────────────────────────────────────────────────────────────────────

type PiiPattern =
  | 'Email Address'
  | 'Phone Number'
  | 'SSN'
  | 'Credit Card'
  | 'API Key'
  | string

type PiiAction = 'masked' | 'detected'

interface PiiEventRow {
  id?: string
  time: string
  agent: string
  pattern: PiiPattern
  action: PiiAction
  model: string
}

// ── Pattern config ─────────────────────────────────────────────────────────────

const PATTERN_STYLE: Record<string, { badge: string; bar: string }> = {
  'Email Address': {
    badge: 'bg-violet-500/15 text-violet-300 ring-1 ring-violet-500/25',
    bar: '#8b5cf6',
  },
  'Phone Number': {
    badge: 'bg-sky-500/15 text-sky-300 ring-1 ring-sky-500/25',
    bar: '#0ea5e9',
  },
  'SSN': {
    badge: 'bg-rose-500/15 text-rose-300 ring-1 ring-rose-500/25',
    bar: '#f43f5e',
  },
  'Credit Card': {
    badge: 'bg-amber-500/15 text-amber-300 ring-1 ring-amber-500/25',
    bar: '#f59e0b',
  },
  'API Key': {
    badge: 'bg-teal-500/15 text-teal-300 ring-1 ring-teal-500/25',
    bar: '#14b8a6',
  },
}

const DEFAULT_PATTERN_STYLE = {
  badge: 'bg-slate-500/15 text-slate-300 ring-1 ring-slate-500/25',
  bar: '#64748b',
}

function getPatternStyle(pattern: string) {
  return PATTERN_STYLE[pattern] ?? DEFAULT_PATTERN_STYLE
}

// ── Stat card ──────────────────────────────────────────────────────────────────

interface StatCardProps {
  icon: React.ReactNode
  label: string
  value: string | number
  sub?: string
  accent?: string
}

function StatCard({ icon, label, value, sub, accent = 'text-white' }: StatCardProps) {
  return (
    <div className="card-interactive p-5 flex items-start gap-4">
      <div className="flex items-center justify-center w-10 h-10 rounded-lg bg-[var(--govrix-surface-elevated)] shrink-0">
        {icon}
      </div>
      <div className="min-w-0">
        <div className="section-label mb-1.5">{label}</div>
        <div className={clsx('stat-value text-3xl', accent)}>{value}</div>
        {sub && <div className="text-[0.6875rem] text-slate-500 mt-1">{sub}</div>}
      </div>
    </div>
  )
}

// ── Bar chart tooltip ──────────────────────────────────────────────────────────

interface BarTooltipProps {
  active?: boolean
  payload?: Array<{ value: number; payload: { pattern: string } }>
  label?: string
}

function BarTooltip({ active, payload }: BarTooltipProps) {
  if (!active || !payload?.length) return null
  const item = payload[0]
  const pattern = item.payload.pattern
  const style = getPatternStyle(pattern)
  return (
    <div className="bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border-strong)] rounded-lg px-3 py-2.5 shadow-xl text-xs">
      <div className={clsx('inline-flex items-center px-2 py-0.5 rounded-full text-[0.6875rem] font-semibold mb-1.5', style.badge)}>
        {pattern}
      </div>
      <div className="flex items-center gap-2">
        <span className="text-slate-400">Detections:</span>
        <span className="font-display font-semibold text-white">{item.value}</span>
      </div>
    </div>
  )
}

// ── Pattern bar chart ──────────────────────────────────────────────────────────

function PatternChart({ data }: { data: { pattern: string; count: number }[] }) {
  return (
    <div className="card p-5 flex flex-col gap-4">
      <div>
        <h2 className="text-sm font-display font-semibold text-[var(--govrix-text-primary)]">
          Detection by Pattern
        </h2>
        <p className="text-xs text-slate-500 mt-0.5">Total matches per PII type — last 24h</p>
      </div>
      <ResponsiveContainer width="100%" height={220}>
        <BarChart
          data={data}
          layout="vertical"
          margin={{ top: 0, right: 24, left: 0, bottom: 0 }}
        >
          <XAxis
            type="number"
            tick={{ fontSize: 11, fill: '#475569', fontFamily: 'Sora' }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis
            type="category"
            dataKey="pattern"
            width={108}
            tick={{ fontSize: 11, fill: '#94a3b8', fontFamily: 'JetBrains Mono' }}
            axisLine={false}
            tickLine={false}
          />
          <Tooltip content={<BarTooltip />} cursor={{ fill: 'rgba(148,163,184,0.04)' }} />
          <Bar dataKey="count" radius={[0, 4, 4, 0]}>
            {data.map((entry) => (
              <Cell
                key={entry.pattern}
                fill={getPatternStyle(entry.pattern).bar}
                fillOpacity={0.85}
              />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
      {/* Legend */}
      <div className="flex flex-wrap gap-2 pt-1">
        {data.map((entry) => (
          <div key={entry.pattern} className="flex items-center gap-1.5">
            <span
              className="w-2 h-2 rounded-full shrink-0"
              style={{ background: getPatternStyle(entry.pattern).bar }}
            />
            <span className="text-[0.6875rem] text-slate-400">{entry.pattern}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

// ── Pattern badge ──────────────────────────────────────────────────────────────

function PatternBadge({ pattern }: { pattern: string }) {
  const style = getPatternStyle(pattern)
  return (
    <span
      className={clsx(
        'inline-flex items-center px-2 py-0.5 rounded-full text-[0.6875rem] font-semibold font-mono',
        style.badge,
      )}
    >
      {pattern}
    </span>
  )
}

// ── Action badge ───────────────────────────────────────────────────────────────

function ActionBadge({ action }: { action: PiiAction }) {
  if (action === 'masked') {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[0.6875rem] font-semibold bg-brand-500/15 text-brand-400 ring-1 ring-brand-500/25">
        <span className="w-1.5 h-1.5 rounded-full bg-brand-500" />
        Masked
      </span>
    )
  }
  return (
    <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[0.6875rem] font-semibold bg-amber-500/15 text-amber-400 ring-1 ring-amber-500/25">
      <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
      Detected
    </span>
  )
}

// ── Events table ───────────────────────────────────────────────────────────────

function EventsTable({ events }: { events: PiiEventRow[] }) {
  return (
    <div className="card overflow-hidden">
      <div className="px-5 py-4 border-b border-[var(--govrix-border)] flex items-center justify-between">
        <div>
          <h2 className="text-sm font-display font-semibold text-[var(--govrix-text-primary)]">
            Recent PII Events
          </h2>
          <p className="text-xs text-slate-500 mt-0.5">
            Latest detections across all agents — today
          </p>
        </div>
        <div className="flex items-center gap-1.5 text-xs text-slate-500 font-mono">
          <span className="w-1.5 h-1.5 rounded-full bg-brand-500 pulse-glow" />
          Live feed
        </div>
      </div>
      <div className="overflow-x-auto">
        <table className="govrix-table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Agent</th>
              <th>Pattern Type</th>
              <th>Action</th>
              <th>Model</th>
            </tr>
          </thead>
          <tbody>
            {events.map((event, idx) => (
              <tr key={event.id ?? `${event.time}-${idx}`}>
                <td>
                  <span className="font-mono text-[0.75rem] text-slate-400">{event.time}</span>
                </td>
                <td>
                  <span className="font-mono text-[0.75rem] text-brand-400">{event.agent}</span>
                </td>
                <td>
                  <PatternBadge pattern={event.pattern} />
                </td>
                <td>
                  <ActionBadge action={event.action} />
                </td>
                <td>
                  <span className="font-mono text-[0.75rem] text-slate-400">{event.model}</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="px-5 py-3 border-t border-[var(--govrix-border)] flex items-center justify-between">
        <span className="text-[0.6875rem] text-slate-500">
          Showing {events.length} most recent events
        </span>
        <span className="text-[0.6875rem] text-slate-500 font-mono">
          PII values are never stored &mdash; type and location only
        </span>
      </div>
    </div>
  )
}

// ── Page ───────────────────────────────────────────────────────────────────────

export function PiiActivityPage() {
  const { data: apiData, isLoading, error } = useQuery({
    queryKey: ['pii-activity'],
    queryFn: fetchPiiActivity,
    staleTime: 30_000,
    retry: 1,
  })

  // Enterprise-only feature: 404
  if (!isLoading && error && isNotFoundError(error)) {
    return (
      <div className="space-y-6 page-enter">
        <div>
          <h1 className="text-lg font-display font-bold text-[var(--govrix-text-primary)] tracking-tight">
            PII Activity
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Data privacy monitoring
          </p>
        </div>
        <div className="card">
          <EnterpriseFeatureCard
            icon={ScanEye}
            title="PII Monitoring requires Govrix Enterprise"
            description="PII activity monitoring provides real-time detection and masking telemetry across all your AI agents. It tracks pattern matches for SSN, email, credit card, and other sensitive data types. Upgrade to Govrix Enterprise to enable this feature."
          />
        </div>
      </div>
    )
  }

  // Real error
  if (!isLoading && error) {
    return (
      <div className="space-y-6 page-enter">
        <div>
          <h1 className="text-lg font-display font-bold text-[var(--govrix-text-primary)] tracking-tight">
            PII Activity
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Data privacy monitoring
          </p>
        </div>
        <div className="card p-8 text-center">
          <p className="text-sm text-red-400">Failed to load PII data: {error.message}</p>
        </div>
      </div>
    )
  }

  const totalDetections = apiData?.total_detections ?? 0
  const maskedCount = apiData?.masked_count ?? 0
  const patternCounts: { pattern: string; count: number }[] = apiData?.pattern_counts ?? []
  const recentEvents: PiiEventRow[] = apiData?.recent_events ?? []
  const uniquePatterns = patternCounts.length
  const maskedPct = totalDetections > 0 ? Math.round((maskedCount / totalDetections) * 100) : 0

  return (
    <div className="space-y-6 page-enter">
      {/* Page header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-lg font-display font-bold text-[var(--govrix-text-primary)] tracking-tight">
            PII Activity
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Data privacy monitoring &mdash; pattern detection &amp; masking telemetry
          </p>
        </div>
        <div
          className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-[0.6875rem] font-semibold font-display"
          style={{ background: 'rgba(16,185,129,0.08)', color: '#34d399', border: '1px solid rgba(16,185,129,0.15)' }}
        >
          <ShieldCheck className="w-3.5 h-3.5" />
          PII Masking Active
        </div>
      </div>

      {/* Stat cards */}
      {isLoading ? (
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="skeleton h-28 rounded-xl" />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 stagger-in">
          <StatCard
            icon={<ScanEye className="w-5 h-5 text-violet-400" />}
            label="Total PII Detections"
            value={totalDetections.toLocaleString()}
            sub="Pattern matches in last 24h"
            accent="text-white"
          />
          <StatCard
            icon={<ShieldCheck className="w-5 h-5 text-brand-400" />}
            label="Masked Successfully"
            value={maskedCount.toLocaleString()}
            sub={`${maskedPct}% of all detections masked before upstream`}
            accent="text-brand-400"
          />
          <StatCard
            icon={<Layers className="w-5 h-5 text-teal-400" />}
            label="Unique Patterns"
            value={uniquePatterns}
            sub="Distinct PII types being tracked"
            accent="text-white"
          />
        </div>
      )}

      {/* Bar chart */}
      {isLoading ? (
        <div className="skeleton h-72 rounded-xl" />
      ) : patternCounts.length > 0 ? (
        <PatternChart data={patternCounts} />
      ) : null}

      {/* Events table */}
      {isLoading ? (
        <div className="card p-5 space-y-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="skeleton h-10 rounded-lg" />
          ))}
        </div>
      ) : recentEvents.length > 0 ? (
        <EventsTable events={recentEvents} />
      ) : !isLoading && totalDetections === 0 ? (
        <div className="card p-10 text-center">
          <p className="text-sm text-slate-400">No PII events detected yet.</p>
          <p className="text-xs text-slate-500 mt-1">PII events will appear when agents send data through the proxy that matches detection patterns.</p>
        </div>
      ) : null}
    </div>
  )
}
