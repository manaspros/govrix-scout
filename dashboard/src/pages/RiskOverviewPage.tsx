import { useMemo } from 'react'
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from 'recharts'
import {
  ShieldAlert,
  Bell,
  AlertTriangle,
  ScanEye,
  Shield,
  Lock,
  Fingerprint,
  DollarSign,
  ClipboardList,
  ScrollText,
  CheckCircle2,
  XCircle,
} from 'lucide-react'
import { clsx } from 'clsx'
import { useQuery } from '@tanstack/react-query'
import { fetchPlatformHealth, fetchRiskOverview } from '@/api/platform'
import type { PlatformHealth, RiskAlert } from '@/api/types'
import { EnterpriseFeatureCard, isNotFoundError } from '@/components/common/EnterpriseFeatureCard'

type Severity = 'critical' | 'high' | 'medium' | 'low'

type Alert = RiskAlert

// ── Utility ────────────────────────────────────────────────────────────────────

function riskScoreColor(score: number): { text: string; ring: string; glow: string; label: string } {
  if (score < 30) return {
    text: 'text-brand-400',
    ring: '#10b981',
    glow: 'rgba(16, 185, 129, 0.25)',
    label: 'Low Risk',
  }
  if (score <= 70) return {
    text: 'text-amber-400',
    ring: '#f59e0b',
    glow: 'rgba(245, 158, 11, 0.25)',
    label: 'Moderate Risk',
  }
  return {
    text: 'text-rose-400',
    ring: '#f43f5e',
    glow: 'rgba(244, 63, 94, 0.25)',
    label: 'High Risk',
  }
}

const SEVERITY_CONFIG: Record<Severity, { dot: string; badge: string; label: string }> = {
  critical: { dot: 'bg-rose-500', badge: 'severity-bg-critical severity-critical border', label: 'Critical' },
  high:     { dot: 'bg-amber-500', badge: 'severity-bg-high severity-high border', label: 'High' },
  medium:   { dot: 'bg-blue-500', badge: 'severity-bg-medium severity-medium border', label: 'Medium' },
  low:      { dot: 'bg-slate-500', badge: 'severity-bg-low severity-low border', label: 'Low' },
}

// ── Sub-components ─────────────────────────────────────────────────────────────

function SeverityBadge({ severity }: { severity: Severity }) {
  const cfg = SEVERITY_CONFIG[severity]
  return (
    <span
      className={clsx(
        'inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[0.6875rem] font-semibold font-display tracking-wide',
        cfg.badge,
      )}
    >
      <span className={clsx('w-1.5 h-1.5 rounded-full', cfg.dot)} />
      {cfg.label}
    </span>
  )
}

// ── Risk Score Gauge ───────────────────────────────────────────────────────────

function RiskGauge({ score }: { score: number }) {
  const { text, ring, glow, label } = riskScoreColor(score)
  // conic-gradient: 0deg = top. We want to fill from 0 to (score/100)*360 degrees.
  const deg = (score / 100) * 360

  return (
    <div className="flex flex-col items-center justify-center gap-2">
      <div
        className="relative flex items-center justify-center"
        style={{ width: 96, height: 96 }}
      >
        {/* Track */}
        <div
          className="absolute inset-0 rounded-full"
          style={{
            background: 'conic-gradient(from 0deg, var(--govrix-surface-elevated) 0deg, var(--govrix-surface-elevated) 360deg)',
          }}
        />
        {/* Fill arc */}
        <div
          className="absolute inset-0 rounded-full transition-all duration-700"
          style={{
            background: `conic-gradient(from 0deg, ${ring} 0deg, ${ring} ${deg}deg, transparent ${deg}deg)`,
            filter: `drop-shadow(0 0 8px ${glow})`,
          }}
        />
        {/* Inner circle mask */}
        <div
          className="absolute rounded-full"
          style={{
            inset: 10,
            background: 'var(--govrix-surface)',
          }}
        />
        {/* Score number */}
        <div className="relative z-10 flex flex-col items-center">
          <span className={clsx('stat-value text-2xl', text)}>{score}</span>
          <span className="text-[0.5rem] text-slate-500 font-display uppercase tracking-widest mt-0.5">/100</span>
        </div>
      </div>
      <div className={clsx('text-xs font-semibold font-display tracking-wide', text)}>{label}</div>
    </div>
  )
}

// ── Stat Cards ─────────────────────────────────────────────────────────────────

function RiskScoreCard({ score }: { score: number }) {
  return (
    <div className="card-interactive p-5 flex flex-col gap-3">
      <div className="flex items-center gap-2">
        <ShieldAlert className="w-4 h-4 text-slate-500" />
        <span className="section-label">Risk Score</span>
      </div>
      <div className="flex items-center justify-center py-1">
        <RiskGauge score={score} />
      </div>
      <p className="text-[0.6875rem] text-slate-500 text-center">
        Composite score from policy, budget &amp; identity signals
      </p>
    </div>
  )
}

interface AlertCountCardProps {
  total: number
  critical: number
  high: number
  medium: number
  low: number
}

function AlertCountCard({ total, critical, high, medium, low }: AlertCountCardProps) {
  return (
    <div className="card-interactive p-5 flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <Bell className="w-4 h-4 text-slate-500" />
        <span className="section-label">Active Alerts</span>
      </div>
      <div className="flex items-end gap-3">
        <span className="stat-value text-4xl text-white">{total}</span>
        <span className="text-slate-500 text-sm mb-1.5">open</span>
      </div>
      <div className="grid grid-cols-4 gap-1.5">
        {([
          { label: 'Critical', count: critical, color: 'bg-rose-500/15 text-rose-400 ring-rose-500/20' },
          { label: 'High',     count: high,     color: 'bg-amber-500/15 text-amber-400 ring-amber-500/20' },
          { label: 'Medium',   count: medium,   color: 'bg-blue-500/15 text-blue-400 ring-blue-500/20' },
          { label: 'Low',      count: low,      color: 'bg-slate-500/10 text-slate-400 ring-slate-500/15' },
        ] as const).map(({ label, count, color }) => (
          <div
            key={label}
            className={clsx('flex flex-col items-center rounded-lg py-2 ring-1', color)}
          >
            <span className="font-display font-semibold text-sm tabular-nums">{count}</span>
            <span className="text-[0.5625rem] opacity-70 tracking-wide">{label}</span>
          </div>
        ))}
      </div>
    </div>
  )
}

function PolicyViolationsCard({ count }: { count: number }) {
  const isElevated = count > 10
  return (
    <div className="card-interactive p-5 flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <AlertTriangle className="w-4 h-4 text-slate-500" />
        <span className="section-label">Policy Violations</span>
      </div>
      <div className="flex flex-col gap-1">
        <span className={clsx('stat-value text-4xl', isElevated ? 'text-amber-400' : 'text-white')}>
          {count}
        </span>
        <span className="text-xs text-slate-500">violations in last 24h</span>
      </div>
      <div className="progress-track">
        <div
          className="progress-fill"
          style={{
            width: `${Math.min(count * 5, 100)}%`,
            background: isElevated ? '#f59e0b' : '#10b981',
          }}
        />
      </div>
      <p className="text-[0.6875rem] text-slate-500">
        {isElevated ? 'Above normal threshold — review policies' : 'Within acceptable range'}
      </p>
    </div>
  )
}

function PiiDetectionsCard({ count }: { count: number }) {
  return (
    <div className="card-interactive p-5 flex flex-col gap-4">
      <div className="flex items-center gap-2">
        <ScanEye className="w-4 h-4 text-slate-500" />
        <span className="section-label">PII Detections</span>
      </div>
      <div className="flex flex-col gap-1">
        <span className="stat-value text-4xl text-white">{count}</span>
        <span className="text-xs text-slate-500">patterns matched in 24h</span>
      </div>
      <div className="flex items-center gap-1.5">
        <span className="w-1.5 h-1.5 rounded-full bg-brand-500 pulse-glow" />
        <span className="text-[0.6875rem] text-brand-400 font-medium">PII masking active</span>
      </div>
      <p className="text-[0.6875rem] text-slate-500">
        All detections masked before reaching upstream models
      </p>
    </div>
  )
}

// ── Risk Trend Chart ───────────────────────────────────────────────────────────

interface TrendTooltipProps {
  active?: boolean
  payload?: Array<{ value: number; color: string }>
  label?: string
}

function TrendTooltip({ active, payload, label }: TrendTooltipProps) {
  if (!active || !payload?.length) return null
  const score = payload[0].value
  const { text, label: riskLabel } = riskScoreColor(score)
  return (
    <div className="bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border-strong)] rounded-lg px-3 py-2 shadow-xl text-xs">
      <div className="text-slate-400 mb-1">{label}</div>
      <div className="flex items-center gap-2">
        <span className={clsx('font-display font-semibold', text)}>{score}</span>
        <span className="text-slate-500">{riskLabel}</span>
      </div>
    </div>
  )
}

function RiskTrendChart({ data }: { data: { day: string; score: number }[] }) {
  return (
    <div className="card p-5 flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold text-[var(--govrix-text-primary)] font-display">Risk Trend</h2>
          <p className="text-xs text-slate-500 mt-0.5">Last 7 days — composite risk score</p>
        </div>
        <div className="flex items-center gap-1.5 text-xs text-slate-500">
          <span className="w-2 h-2 rounded-full bg-amber-400" />
          Risk Score
        </div>
      </div>
      <ResponsiveContainer width="100%" height={180}>
        <LineChart data={data} margin={{ top: 4, right: 4, left: -24, bottom: 0 }}>
          <defs>
            <linearGradient id="riskGrad" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#f59e0b" stopOpacity={0.2} />
              <stop offset="100%" stopColor="#f59e0b" stopOpacity={0} />
            </linearGradient>
          </defs>
          <CartesianGrid strokeDasharray="3 3" stroke="rgba(148,163,184,0.06)" />
          <XAxis
            dataKey="day"
            tick={{ fontSize: 11, fill: '#475569', fontFamily: 'Sora' }}
            axisLine={false}
            tickLine={false}
          />
          <YAxis
            domain={[0, 100]}
            tick={{ fontSize: 11, fill: '#475569', fontFamily: 'Sora' }}
            axisLine={false}
            tickLine={false}
          />
          <Tooltip content={<TrendTooltip />} cursor={{ stroke: 'rgba(148,163,184,0.1)', strokeWidth: 1 }} />
          <Line
            type="monotone"
            dataKey="score"
            stroke="#f59e0b"
            strokeWidth={2}
            dot={{ fill: '#f59e0b', r: 3, strokeWidth: 0 }}
            activeDot={{ fill: '#f59e0b', r: 5, strokeWidth: 2, stroke: 'rgba(245,158,11,0.3)' }}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  )
}

// ── Alert List ─────────────────────────────────────────────────────────────────

function AlertList({ alerts }: { alerts: Alert[] }) {
  return (
    <div className="card flex flex-col">
      <div className="px-5 py-4 border-b border-[var(--govrix-border)]">
        <h2 className="text-sm font-semibold text-[var(--govrix-text-primary)] font-display">Active Alerts</h2>
        <p className="text-xs text-slate-500 mt-0.5">Most recent — sorted by severity</p>
      </div>
      <div className="divide-y divide-[var(--govrix-border)]">
        {alerts.map((alert) => (
          <div
            key={alert.id}
            className="px-5 py-3.5 flex items-start gap-3 hover:bg-[var(--govrix-accent-glow)] transition-colors duration-150"
          >
            <div className="mt-0.5 shrink-0">
              <SeverityBadge severity={alert.severity} />
            </div>
            <div className="min-w-0 flex-1">
              <p className="text-xs text-[var(--govrix-text-primary)] leading-relaxed line-clamp-2">
                {alert.message}
              </p>
              <div className="flex items-center gap-2 mt-1.5">
                <span className="font-mono text-[0.6875rem] text-brand-400">{alert.agent}</span>
                <span className="text-slate-600">·</span>
                <span className="text-[0.6875rem] text-slate-500">{alert.timestamp}</span>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}

// ── Security Posture ───────────────────────────────────────────────────────────

interface PostureFeature {
  icon: React.ReactNode
  label: string
  enabled: boolean
  description: string
}

function buildPostureFeatures(health: PlatformHealth | undefined): PostureFeature[] {
  return [
    {
      icon: <ScrollText className="w-5 h-5" />,
      label: 'Policy Engine',
      enabled: health?.policy_enabled ?? false,
      description: 'YAML rule evaluation on all traffic',
    },
    {
      icon: <ScanEye className="w-5 h-5" />,
      label: 'PII Masking',
      enabled: health?.pii_masking_enabled ?? false,
      description: 'Outbound payload scrubbing active',
    },
    {
      icon: <Fingerprint className="w-5 h-5" />,
      label: 'mTLS Identity',
      enabled: health?.mtls_enabled ?? false,
      description: 'X.509 cert-bound agent authentication',
    },
    {
      icon: <DollarSign className="w-5 h-5" />,
      label: 'Budget Tracking',
      enabled: health?.budget_tracking_enabled ?? false,
      description: 'Per-agent & global spend enforcement',
    },
    {
      icon: <ClipboardList className="w-5 h-5" />,
      label: 'Compliance Monitor',
      enabled: health?.compliance_enabled ?? false,
      description: 'SOC 2, HIPAA, EU AI Act controls',
    },
    {
      icon: <Shield className="w-5 h-5" />,
      label: 'Audit Trail',
      enabled: health?.audit_trail_enabled ?? false,
      description: 'SHA-256 integrity-signed session log',
    },
  ]
}

function PostureCard({ feature }: { feature: PostureFeature }) {
  return (
    <div
      className={clsx(
        'card-interactive p-4 flex items-start gap-3',
        !feature.enabled && 'opacity-60',
      )}
    >
      <div
        className={clsx(
          'flex items-center justify-center w-9 h-9 rounded-lg shrink-0',
          feature.enabled
            ? 'bg-brand-500/10 text-brand-400'
            : 'bg-slate-500/10 text-slate-500',
        )}
      >
        {feature.icon}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[0.8125rem] font-semibold text-[var(--govrix-text-primary)] font-display">
            {feature.label}
          </span>
          <div className="flex items-center gap-1.5 shrink-0">
            {feature.enabled ? (
              <CheckCircle2 className="w-4 h-4 text-brand-500" />
            ) : (
              <XCircle className="w-4 h-4 text-slate-600" />
            )}
            <span
              className={clsx(
                'text-[0.625rem] font-display font-semibold uppercase tracking-wider',
                feature.enabled ? 'text-brand-400' : 'text-slate-500',
              )}
            >
              {feature.enabled ? 'Active' : 'Off'}
            </span>
          </div>
        </div>
        <p className="text-[0.6875rem] text-slate-500 mt-0.5 leading-relaxed">
          {feature.description}
        </p>
      </div>
    </div>
  )
}

// ── Lock icon for posture section ──────────────────────────────────────────────

function PostureSection({ health, isLoading }: { health: PlatformHealth | undefined; isLoading: boolean }) {
  const features = useMemo(() => buildPostureFeatures(health), [health])
  const enabledCount = features.filter(f => f.enabled).length

  return (
    <div className="card">
      <div className="px-5 py-4 border-b border-[var(--govrix-border)] flex items-center justify-between">
        <div>
          <div className="flex items-center gap-2">
            <Lock className="w-4 h-4 text-slate-500" />
            <h2 className="text-sm font-semibold text-[var(--govrix-text-primary)] font-display">Security Posture</h2>
          </div>
          <p className="text-xs text-slate-500 mt-0.5">Enterprise protection layer status</p>
        </div>
        <div className="flex items-center gap-2">
          <div className="progress-track w-24">
            <div
              className="progress-fill bg-brand-500"
              style={{ width: `${(enabledCount / features.length) * 100}%` }}
            />
          </div>
          <span className="text-xs font-display font-semibold text-brand-400">
            {enabledCount}/{features.length}
          </span>
        </div>
      </div>
      <div className="p-4">
        {isLoading ? (
          <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <div key={i} className="skeleton h-20 rounded-lg" />
            ))}
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 gap-3 stagger-in">
            {features.map((feature) => (
              <PostureCard key={feature.label} feature={feature} />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

// ── Page ───────────────────────────────────────────────────────────────────────

export function RiskOverviewPage() {
  const { data: health, isLoading: healthLoading, error: healthError } = useQuery({
    queryKey: ['platform-health'],
    queryFn: fetchPlatformHealth,
    staleTime: 30_000,
    retry: 1,
  })

  const { data: riskData, isLoading: riskLoading, error: riskError } = useQuery({
    queryKey: ['risk-overview'],
    queryFn: fetchRiskOverview,
    staleTime: 30_000,
    retry: 1,
  })

  // If the risk overview API returned a 404, this is an enterprise-only feature
  if (!riskLoading && riskError && isNotFoundError(riskError)) {
    return (
      <div className="space-y-6 page-enter">
        <div>
          <h1 className="text-lg font-display font-bold text-[var(--govrix-text-primary)] tracking-tight">
            Risk Overview
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Enterprise security command center
          </p>
        </div>
        <div className="card">
          <EnterpriseFeatureCard
            icon={ShieldAlert}
            title="Risk Monitoring requires Govrix Enterprise"
            description="The risk overview dashboard provides real-time risk scoring, alert management, and trend analysis across all your AI agents. Upgrade to Govrix Enterprise to enable this feature."
          />
        </div>
        {/* Still show security posture if health endpoint works */}
        {!healthError && <PostureSection health={health} isLoading={healthLoading} />}
      </div>
    )
  }

  // If there's a real (non-404) error
  if (!riskLoading && riskError) {
    return (
      <div className="space-y-6 page-enter">
        <div>
          <h1 className="text-lg font-display font-bold text-[var(--govrix-text-primary)] tracking-tight">
            Risk Overview
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Enterprise security command center
          </p>
        </div>
        <div className="card p-8 text-center">
          <p className="text-sm text-red-400">Failed to load risk data: {riskError.message}</p>
        </div>
      </div>
    )
  }

  // Data loaded successfully
  const riskScore = riskData?.risk_score ?? 0
  const trendData = riskData?.trend ?? []
  const alerts: Alert[] = riskData?.alerts ?? []
  const stats = riskData?.stats ?? {
    total_alerts: 0, critical: 0, high: 0, medium: 0, low: 0,
    policy_violations_24h: 0, pii_detections_24h: 0,
  }

  return (
    <div className="space-y-6 page-enter">
      {/* Page header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-lg font-display font-bold text-[var(--govrix-text-primary)] tracking-tight">
            Risk Overview
          </h1>
          <p className="text-xs text-slate-500 mt-0.5">
            Enterprise security command center &mdash; live signals
          </p>
        </div>
        <div className="flex items-center gap-2 text-xs text-slate-500 font-mono">
          <span className="w-1.5 h-1.5 rounded-full bg-brand-500 pulse-glow" />
          Live
        </div>
      </div>

      {/* Stat cards row */}
      {riskLoading ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="skeleton h-40 rounded-xl" />
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 stagger-in">
          <RiskScoreCard score={riskScore} />
          <AlertCountCard
            total={stats.total_alerts}
            critical={stats.critical}
            high={stats.high}
            medium={stats.medium}
            low={stats.low}
          />
          <PolicyViolationsCard count={stats.policy_violations_24h} />
          <PiiDetectionsCard count={stats.pii_detections_24h} />
        </div>
      )}

      {/* Middle row: trend + alert list */}
      {trendData.length > 0 && (
        <div className="grid grid-cols-1 xl:grid-cols-2 gap-4">
          <RiskTrendChart data={trendData} />
          <AlertList alerts={alerts} />
        </div>
      )}
      {trendData.length === 0 && alerts.length > 0 && (
        <AlertList alerts={alerts} />
      )}

      {/* Security posture grid */}
      <PostureSection health={health} isLoading={healthLoading} />
    </div>
  )
}
