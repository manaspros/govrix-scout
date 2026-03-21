import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { format } from 'date-fns'
import { ChevronDown, ChevronUp, ClipboardList } from 'lucide-react'
import { clsx } from 'clsx'
import { fetchComplianceReport } from '@/api/platform'
import type { ComplianceReport, ComplianceControl } from '@/api/types'
import { EnterpriseFeatureCard, isNotFoundError } from '@/components/common/EnterpriseFeatureCard'

// ── Framework definitions ──────────────────────────────────────────────────────

interface FrameworkDef {
  key: string
  label: string
  shortLabel: string
  color: string
  accentColor: string
}

const FRAMEWORKS: FrameworkDef[] = [
  { key: 'soc2',       label: 'SOC 2 Type II', shortLabel: 'SOC 2',    color: 'text-sky-400',    accentColor: '#38bdf8' },
  { key: 'eu-ai-act',  label: 'EU AI Act',     shortLabel: 'EU AI Act', color: 'text-violet-400', accentColor: '#a78bfa' },
  { key: 'hipaa',      label: 'HIPAA',          shortLabel: 'HIPAA',    color: 'text-amber-400',  accentColor: '#fbbf24' },
  { key: 'nist-800-53',label: 'NIST 800-53',   shortLabel: 'NIST',     color: 'text-rose-400',   accentColor: '#fb7185' },
]

// ── Score gauge ───────────────────────────────────────────────────────────────

interface ScoreGaugeProps {
  score: number   // 0–1
  size?: number
  accentColor: string
  selected: boolean
}

function ScoreGauge({ score, size = 64, accentColor, selected }: ScoreGaugeProps) {
  const pct = Math.round(score * 100)
  const bg = selected ? accentColor : '#334155'

  return (
    <div
      className="relative flex items-center justify-center rounded-full shrink-0"
      style={{
        width: size,
        height: size,
        background: `conic-gradient(${bg} ${pct}%, #1e293b ${pct}% 100%)`,
        boxShadow: selected ? `0 0 0 3px rgba(0,0,0,0.6), 0 0 0 4px ${accentColor}33` : undefined,
      }}
    >
      {/* Inner circle mask */}
      <div
        className="absolute rounded-full bg-[var(--govrix-surface)]"
        style={{ width: size - 10, height: size - 10 }}
      />
      <span className="relative z-10 font-mono font-bold text-sm text-[var(--govrix-text-primary)]">
        {pct}%
      </span>
    </div>
  )
}

// ── Status badge ──────────────────────────────────────────────────────────────

const STATUS_MAP: Record<ComplianceControl['status'], { label: string; dot: string; pill: string }> = {
  pass:           { label: 'Pass',    dot: 'bg-emerald-400', pill: 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20' },
  fail:           { label: 'Fail',    dot: 'bg-rose-400',    pill: 'bg-rose-500/10 text-rose-400 border border-rose-500/20' },
  partial:        { label: 'Partial', dot: 'bg-amber-400',   pill: 'bg-amber-500/10 text-amber-400 border border-amber-500/20' },
  not_applicable: { label: 'N/A',     dot: 'bg-slate-500',   pill: 'bg-slate-500/10 text-slate-400 border border-slate-500/20' },
}

function StatusBadge({ status }: { status: ComplianceControl['status'] }) {
  const cfg = STATUS_MAP[status]
  return (
    <span className={clsx('inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium whitespace-nowrap', cfg.pill)}>
      <span className={clsx('w-1.5 h-1.5 rounded-full shrink-0', cfg.dot)} />
      {cfg.label}
    </span>
  )
}

// ── Control row ───────────────────────────────────────────────────────────────

interface ControlRowProps {
  control: ComplianceControl
}

function ControlRow({ control }: ControlRowProps) {
  const [expanded, setExpanded] = useState(false)

  return (
    <tr
      className="cursor-pointer"
      onClick={() => setExpanded(e => !e)}
    >
      <td className="w-24">
        <code className="font-mono text-xs font-semibold text-brand-400 bg-[var(--govrix-surface-elevated)] px-2 py-0.5 rounded whitespace-nowrap">
          {control.id}
        </code>
      </td>
      <td>
        <div className="text-sm font-medium text-[var(--govrix-text-primary)]">{control.name}</div>
        <div className="text-xs text-[var(--govrix-text-muted)] mt-0.5 line-clamp-1">{control.description}</div>
      </td>
      <td className="w-24">
        <StatusBadge status={control.status} />
      </td>
      <td className="max-w-xs">
        {expanded ? (
          <div className="space-y-1">
            <p className="text-xs text-slate-400 leading-relaxed">{control.evidence ?? 'No evidence recorded.'}</p>
            <span className="inline-flex items-center gap-1 text-[10px] text-brand-400 mt-1">
              <ChevronUp className="w-3 h-3" /> Collapse
            </span>
          </div>
        ) : (
          <div className="flex items-center gap-1 group">
            <p className="text-xs text-slate-500 truncate max-w-[180px]">{control.evidence ?? '—'}</p>
            <span className="text-[10px] text-brand-400 opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap flex items-center gap-0.5">
              <ChevronDown className="w-3 h-3" /> Show
            </span>
          </div>
        )}
      </td>
    </tr>
  )
}

// ── Compliance page ───────────────────────────────────────────────────────────

export function CompliancePage() {
  const [selectedKey, setSelectedKey] = useState<string>('soc2')

  const { data: apiReport, isLoading, error } = useQuery({
    queryKey: ['platform', 'compliance', selectedKey],
    queryFn: () => fetchComplianceReport(selectedKey),
    retry: 1,
  })

  // Preload all 4 frameworks so the cards show real scores immediately
  const { data: soc2Data, error: soc2Error }     = useQuery({ queryKey: ['platform', 'compliance', 'soc2'],       queryFn: () => fetchComplianceReport('soc2'),       retry: 1 })
  const { data: euAiActData }  = useQuery({ queryKey: ['platform', 'compliance', 'eu-ai-act'],  queryFn: () => fetchComplianceReport('eu-ai-act'),  retry: 1 })
  const { data: hipaaData }    = useQuery({ queryKey: ['platform', 'compliance', 'hipaa'],      queryFn: () => fetchComplianceReport('hipaa'),      retry: 1 })
  const { data: nistData }     = useQuery({ queryKey: ['platform', 'compliance', 'nist-800-53'],queryFn: () => fetchComplianceReport('nist-800-53'),retry: 1 })

  // If the first query (soc2) returns a 404, the whole compliance API is enterprise-only
  if (!isLoading && (error || soc2Error) && isNotFoundError(error ?? soc2Error)) {
    return (
      <div className="space-y-6 page-enter">
        <div>
          <h1 className="font-display text-xl font-semibold text-[var(--govrix-text-primary)] tracking-tight">
            Compliance
          </h1>
          <p className="text-xs text-[var(--govrix-text-muted)] mt-0.5">
            Framework control status
          </p>
        </div>
        <div className="card">
          <EnterpriseFeatureCard
            icon={ClipboardList}
            title="Compliance Monitoring requires Govrix Enterprise"
            description="The compliance dashboard tracks control status across SOC 2, EU AI Act, HIPAA, and NIST 800-53 frameworks. It provides real-time scoring, evidence tracking, and audit-ready reports. Upgrade to Govrix Enterprise to enable this feature."
          />
        </div>
      </div>
    )
  }

  // Real error
  if (!isLoading && error && !isNotFoundError(error)) {
    return (
      <div className="space-y-6 page-enter">
        <div>
          <h1 className="font-display text-xl font-semibold text-[var(--govrix-text-primary)] tracking-tight">
            Compliance
          </h1>
          <p className="text-xs text-[var(--govrix-text-muted)] mt-0.5">
            Framework control status
          </p>
        </div>
        <div className="card p-8 text-center">
          <p className="text-sm text-red-400">Failed to load compliance data: {error.message}</p>
        </div>
      </div>
    )
  }

  const allReports: Record<string, ComplianceReport | undefined> = {
    'soc2': soc2Data,
    'eu-ai-act': euAiActData,
    'hipaa': hipaaData,
    'nist-800-53': nistData,
  }

  const EMPTY_REPORT: ComplianceReport = {
    framework: FRAMEWORKS.find(f => f.key === selectedKey)?.label ?? selectedKey,
    generated_at: new Date().toISOString(),
    overall_score: 0,
    controls: [],
  }

  const report: ComplianceReport = apiReport ?? EMPTY_REPORT

  function cardScore(key: string): number {
    return allReports[key]?.overall_score ?? 0
  }

  function cardControlsSummary(key: string): { pass: number; total: number } {
    const controls = allReports[key]?.controls ?? []
    return { pass: controls.filter(c => c.status === 'pass').length, total: controls.length }
  }

  return (
    <div className="space-y-6 page-enter">

      {/* Page header */}
      <div>
        <h1 className="font-display text-xl font-semibold text-[var(--govrix-text-primary)] tracking-tight">
          Compliance
        </h1>
        <p className="text-xs text-[var(--govrix-text-muted)] mt-0.5">
          Framework control status — SOC 2, EU AI Act, HIPAA, NIST 800-53
        </p>
      </div>

      {/* Framework selector cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4 stagger-in">
        {FRAMEWORKS.map(fw => {
          const { pass, total } = cardControlsSummary(fw.key)
          const score = cardScore(fw.key)
          const active = selectedKey === fw.key

          return (
            <button
              key={fw.key}
              onClick={() => setSelectedKey(fw.key)}
              className={clsx(
                'card-interactive p-5 text-left w-full transition-all',
                active && 'border-emerald-500/30 shadow-[0_0_0_1px_rgba(16,185,129,0.1),0_0_24px_rgba(16,185,129,0.08)]',
              )}
            >
              <div className="flex items-start justify-between mb-4">
                <div>
                  <div className={clsx('section-label mb-1', active && 'text-emerald-500')}>
                    Framework
                  </div>
                  <div className={clsx('font-display font-semibold text-sm leading-snug', fw.color)}>
                    {fw.label}
                  </div>
                </div>
                {active && (
                  <div className="w-2 h-2 rounded-full bg-emerald-400 mt-1 pulse-glow" />
                )}
              </div>

              <div className="flex items-center gap-3">
                <ScoreGauge
                  score={score}
                  size={52}
                  accentColor={active ? '#10b981' : fw.accentColor}
                  selected={active}
                />
                <div>
                  <div className="stat-value text-xl text-[var(--govrix-text-primary)]">
                    {pass}<span className="text-[var(--govrix-text-muted)] font-normal text-sm">/{total}</span>
                  </div>
                  <div className="text-xs text-[var(--govrix-text-muted)] mt-0.5">controls pass</div>
                </div>
              </div>

              {/* Mini progress bar */}
              <div className="progress-track mt-3">
                <div
                  className="progress-fill"
                  style={{
                    width: `${score * 100}%`,
                    background: active ? '#10b981' : fw.accentColor,
                  }}
                />
              </div>
            </button>
          )
        })}
      </div>

      {/* Controls detail */}
      <div className="card overflow-hidden">
        {/* Section header */}
        <div className="px-5 py-4 border-b border-[var(--govrix-border)] flex items-center justify-between">
          <div>
            <h2 className="font-display text-sm font-semibold text-[var(--govrix-text-primary)]">
              {report.framework}
            </h2>
            <div className="flex items-center gap-2 mt-0.5">
              <div className="section-label">
                Generated {report.generated_at ? format(new Date(report.generated_at), 'MMM d, yyyy HH:mm') : 'N/A'}
              </div>
            </div>
          </div>

          {/* Pass/fail summary pills */}
          <div className="flex items-center gap-2">
            {(() => {
              const controls = report.controls ?? []
              const pass = controls.filter(c => c.status === 'pass').length
              const partial = controls.filter(c => c.status === 'partial').length
              const fail = controls.filter(c => c.status === 'fail').length
              return (
                <>
                  <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                    {pass} Pass
                  </span>
                  {partial > 0 && (
                    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-amber-500/10 text-amber-400 border border-amber-500/20">
                      <span className="w-1.5 h-1.5 rounded-full bg-amber-400" />
                      {partial} Partial
                    </span>
                  )}
                  {fail > 0 && (
                    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-rose-500/10 text-rose-400 border border-rose-500/20">
                      <span className="w-1.5 h-1.5 rounded-full bg-rose-400" />
                      {fail} Fail
                    </span>
                  )}
                </>
              )
            })()}
          </div>
        </div>

        {/* Controls table */}
        <div className="overflow-x-auto">
          {isLoading ? (
            <div className="p-6 space-y-3">
              {Array.from({ length: 5 }).map((_, i) => (
                <div key={i} className="skeleton h-10 rounded" />
              ))}
            </div>
          ) : (report.controls ?? []).length === 0 ? (
            <div className="p-10 text-center">
              <p className="text-sm text-slate-400">No compliance controls loaded for this framework.</p>
              <p className="text-xs text-slate-500 mt-1">Controls will appear when the compliance engine is configured.</p>
            </div>
          ) : (
            <table className="govrix-table">
              <thead>
                <tr>
                  <th className="w-28">Control ID</th>
                  <th>Name & Description</th>
                  <th className="w-24">Status</th>
                  <th>Evidence</th>
                </tr>
              </thead>
              <tbody>
                {(report.controls ?? []).map(control => (
                  <ControlRow key={control.id} control={control} />
                ))}
              </tbody>
            </table>
          )}
        </div>

        {/* Footer score bar */}
        <div className="px-5 py-3 border-t border-[var(--govrix-border)] bg-[var(--govrix-surface-elevated)] flex items-center gap-4">
          <span className="section-label shrink-0">Overall Score</span>
          <div className="progress-track flex-1">
            <div
              className="progress-fill bg-emerald-500"
              style={{ width: `${report.overall_score * 100}%` }}
            />
          </div>
          <span className="font-mono text-sm font-semibold text-emerald-400 shrink-0 tabular-nums">
            {Math.round(report.overall_score * 100)}%
          </span>
        </div>
      </div>

    </div>
  )
}
