import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { ShieldCheck, RefreshCw, ChevronDown, ChevronUp, ScrollText } from 'lucide-react'
import { clsx } from 'clsx'
import { fetchPolicies, reloadPolicies } from '@/api/platform'
import type { PolicySummary, PolicyRule } from '@/api/types'
import { EnterpriseFeatureCard, isNotFoundError } from '@/components/common/EnterpriseFeatureCard'

const SAMPLE_YAML = `# Govrix Policy Engine — v1.0
# Hot-reload: POST /api/v1/policies/reload

policy:
  enabled: true
  pii_masking: true

rules:
  - id: rule-001
    priority: 1
    name: "Block PII in Requests"
    conditions:
      - field: pii_detected
        operator: eq
        value: true
    action: block

  - id: rule-002
    priority: 2
    name: "Rate Limit by Agent"
    conditions:
      - field: requests_per_min
        operator: gt
        value: 100
    action: throttle
    throttle_rps: 20

  - id: rule-003
    priority: 3
    name: "Require mTLS for External"
    conditions:
      - field: source
        operator: eq
        value: external
    action: require_mtls

  - id: rule-004
    priority: 4
    name: "Budget Alert at 80%"
    conditions:
      - field: budget_usage
        operator: gt
        value: 0.8
    action: alert
    alert_channel: slack

  - id: rule-007
    priority: 7
    name: "Enforce EU Data Residency"
    conditions:
      - field: tenant_region
        operator: eq
        value: eu
    action: route_eu
    target_cluster: eu-west-1`

// ── Action badge ──────────────────────────────────────────────────────────────

const ACTION_STYLES: Record<string, string> = {
  block:        'bg-rose-500/10 text-rose-400 border border-rose-500/20',
  throttle:     'bg-amber-500/10 text-amber-400 border border-amber-500/20',
  alert:        'bg-sky-500/10 text-sky-400 border border-sky-500/20',
  log:          'bg-slate-500/10 text-slate-400 border border-slate-500/20',
  require_mtls: 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20',
  route_eu:     'bg-violet-500/10 text-violet-400 border border-violet-500/20',
  audit:        'bg-indigo-500/10 text-indigo-400 border border-indigo-500/20',
}

interface ActionBadgeProps {
  action: string
}

function ActionBadge({ action }: ActionBadgeProps) {
  const classes = ACTION_STYLES[action] ?? 'bg-slate-500/10 text-slate-400 border border-slate-500/20'
  return (
    <span className={clsx('inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium font-mono', classes)}>
      {action}
    </span>
  )
}

// ── Priority badge ────────────────────────────────────────────────────────────

function PriorityBadge({ priority }: { priority: number }) {
  return (
    <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border-strong)] text-[10px] font-mono font-semibold text-slate-400 tabular-nums">
      {priority}
    </span>
  )
}

// ── Toggle switch ─────────────────────────────────────────────────────────────

interface ToggleProps {
  active: boolean
  onChange: () => void
}

function Toggle({ active, onChange }: ToggleProps) {
  return (
    <button
      role="switch"
      aria-checked={active}
      onClick={onChange}
      className={clsx('toggle', active && 'active')}
    />
  )
}

// ── YAML viewer ───────────────────────────────────────────────────────────────

function YamlViewer({ yaml }: { yaml: string }) {
  const lines = yaml.split('\n')

  return (
    <div className="font-mono text-[11px] leading-5 overflow-x-auto">
      {lines.map((line, i) => {
        const trimmed = line.trimStart()
        const indent = line.length - trimmed.length

        let colorClass = 'text-slate-400'

        if (trimmed.startsWith('#')) {
          colorClass = 'text-slate-600 italic'
        } else if (trimmed.match(/^[a-z_]+:/) && !trimmed.startsWith('-')) {
          colorClass = 'text-brand-400'
        } else if (trimmed.startsWith('- id:') || trimmed.match(/^- (id|priority|name|conditions|action|field|operator|value|target|alert):/)) {
          colorClass = 'text-slate-300'
        } else if (trimmed.match(/:\s+".+"/) || trimmed.match(/:\s+[a-z].+/)) {
          colorClass = 'text-slate-300'
        }

        return (
          <div key={i} className="whitespace-pre">
            <span className="select-none text-slate-700 mr-3 text-[10px] tabular-nums">{String(i + 1).padStart(3, ' ')}</span>
            <span style={{ paddingLeft: indent * 1 }} className={colorClass}>
              {line.trimStart()}
            </span>
          </div>
        )
      })}
    </div>
  )
}

// ── Policies page ─────────────────────────────────────────────────────────────

export function PoliciesPage() {
  const queryClient = useQueryClient()
  const [yamlOpen, setYamlOpen] = useState(false)
  const [toggleStates, setToggleStates] = useState<Record<string, boolean>>({})

  const { data: apiData, isLoading, error } = useQuery({
    queryKey: ['platform', 'policies'],
    queryFn: fetchPolicies,
    retry: 1,
  })

  // Enterprise-only feature: 404
  if (!isLoading && error && isNotFoundError(error)) {
    return (
      <div className="space-y-6 page-enter">
        <div>
          <h1 className="font-display text-xl font-semibold text-[var(--govrix-text-primary)] tracking-tight">
            Policy Rules
          </h1>
          <p className="text-xs text-[var(--govrix-text-muted)] mt-0.5">
            Governance rules engine
          </p>
        </div>
        <div className="card">
          <EnterpriseFeatureCard
            icon={ScrollText}
            title="Policy Engine requires Govrix Enterprise"
            description="The policy engine evaluates YAML-based governance rules per request at the proxy layer. It supports PII blocking, rate limiting, mTLS enforcement, budget alerts, and data residency routing. Upgrade to Govrix Enterprise to enable this feature."
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
          <h1 className="font-display text-xl font-semibold text-[var(--govrix-text-primary)] tracking-tight">
            Policy Rules
          </h1>
          <p className="text-xs text-[var(--govrix-text-muted)] mt-0.5">
            Governance rules engine
          </p>
        </div>
        <div className="card p-8 text-center">
          <p className="text-sm text-red-400">Failed to load policies: {error.message}</p>
        </div>
      </div>
    )
  }

  const summary: PolicySummary = apiData ?? {
    total_rules: 0, enabled_rules: 0, policy_enabled: false, pii_masking_enabled: false, rules: [],
  }
  const rules: PolicyRule[] = summary.rules ?? []

  // Per-rule toggle state: initialised from API, overridden by local state
  function isEnabled(rule: PolicyRule): boolean {
    return toggleStates[rule.id] !== undefined ? toggleStates[rule.id] : rule.enabled
  }

  function toggleRule(ruleId: string, current: boolean) {
    setToggleStates(prev => ({ ...prev, [ruleId]: !current }))
  }

  const activeCount = rules.filter(r => isEnabled(r)).length

  const { mutate: reload, isPending: reloading } = useMutation({
    mutationFn: () => reloadPolicies(),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['platform', 'policies'] })
    },
  })

  return (
    <div className="space-y-6 page-enter">

      {/* Header bar */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="font-display text-xl font-semibold text-[var(--govrix-text-primary)] tracking-tight">
            Policy Rules
          </h1>
          <p className="text-xs text-[var(--govrix-text-muted)] mt-0.5">
            Governance rules engine — conditions evaluated per request at proxy layer
          </p>
        </div>
        <button
          className="btn btn-primary"
          onClick={() => reload()}
          disabled={reloading}
        >
          <RefreshCw className={clsx('w-3.5 h-3.5', reloading && 'animate-spin')} />
          {reloading ? 'Reloading…' : 'Reload Policies'}
        </button>
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 stagger-in">

        {/* Total rules */}
        <div className="card p-5 flex items-start gap-4">
          <div className="flex items-center justify-center w-9 h-9 rounded-lg bg-slate-700/40 shrink-0">
            <ShieldCheck className="w-4.5 h-4.5 text-slate-400" style={{ width: '18px', height: '18px' }} />
          </div>
          <div>
            <div className="section-label mb-1.5">Total Rules</div>
            {isLoading ? (
              <div className="skeleton h-7 w-8 rounded" />
            ) : (
              <div className="stat-value text-2xl text-[var(--govrix-text-primary)]">{summary.total_rules}</div>
            )}
            <div className="text-xs text-[var(--govrix-text-muted)] mt-0.5">Defined in policy YAML</div>
          </div>
        </div>

        {/* Active rules */}
        <div className="card p-5 flex items-start gap-4">
          <div className="flex items-center justify-center w-9 h-9 rounded-lg bg-emerald-500/10 shrink-0">
            <div className="w-2 h-2 rounded-full bg-emerald-400 pulse-glow" />
          </div>
          <div>
            <div className="section-label mb-1.5">Active Rules</div>
            {isLoading ? (
              <div className="skeleton h-7 w-8 rounded" />
            ) : (
              <div className="stat-value text-2xl text-emerald-400">{activeCount}</div>
            )}
            <div className="text-xs text-[var(--govrix-text-muted)] mt-0.5">Currently enforcing</div>
          </div>
        </div>

        {/* PII Masking */}
        <div className="card p-5 flex items-start gap-4">
          <div className={clsx(
            'flex items-center justify-center w-9 h-9 rounded-lg shrink-0',
            summary.pii_masking_enabled ? 'bg-emerald-500/10' : 'bg-rose-500/10',
          )}>
            <ShieldCheck className={clsx(
              'w-4.5 h-4.5',
              summary.pii_masking_enabled ? 'text-emerald-400' : 'text-rose-400',
            )} style={{ width: '18px', height: '18px' }} />
          </div>
          <div>
            <div className="section-label mb-1.5">PII Masking</div>
            {isLoading ? (
              <div className="skeleton h-7 w-20 rounded" />
            ) : (
              <div className={clsx(
                'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold',
                summary.pii_masking_enabled
                  ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
                  : 'bg-rose-500/10 text-rose-400 border border-rose-500/20',
              )}>
                <span className={clsx(
                  'w-1.5 h-1.5 rounded-full',
                  summary.pii_masking_enabled ? 'bg-emerald-400' : 'bg-rose-400',
                )} />
                {summary.pii_masking_enabled ? 'Enabled' : 'Disabled'}
              </div>
            )}
            <div className="text-xs text-[var(--govrix-text-muted)] mt-1">Regex-based, 5 patterns</div>
          </div>
        </div>

      </div>

      {/* Rules table */}
      <div className="card overflow-hidden">
        <div className="px-5 py-4 border-b border-[var(--govrix-border)]">
          <div className="flex items-center justify-between">
            <h2 className="font-display text-sm font-semibold text-[var(--govrix-text-primary)]">
              Policy Rules
            </h2>
            <span className="section-label">{rules.length} rules</span>
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="govrix-table">
            <thead>
              <tr>
                <th className="w-16">Priority</th>
                <th>Rule Name</th>
                <th>Conditions</th>
                <th>Action</th>
                <th className="w-24 text-center">Status</th>
              </tr>
            </thead>
            <tbody>
              {isLoading ? (
                Array.from({ length: 5 }).map((_, i) => (
                  <tr key={i}>
                    {Array.from({ length: 5 }).map((__, j) => (
                      <td key={j}>
                        <div className="skeleton h-4 rounded" />
                      </td>
                    ))}
                  </tr>
                ))
              ) : rules.length === 0 ? (
                <tr>
                  <td colSpan={5} className="text-center py-10">
                    <p className="text-sm text-slate-400">No policy rules configured yet.</p>
                    <p className="text-xs text-slate-500 mt-1">Add rules to your policy YAML and reload.</p>
                  </td>
                </tr>
              ) : (
                rules.map(rule => {
                  const enabled = isEnabled(rule)
                  return (
                    <tr key={rule.id}>
                      <td>
                        <PriorityBadge priority={rule.priority} />
                      </td>
                      <td>
                        <span className={clsx(
                          'text-sm font-medium',
                          enabled ? 'text-[var(--govrix-text-primary)]' : 'text-[var(--govrix-text-muted)] line-through',
                        )}>
                          {rule.name}
                        </span>
                      </td>
                      <td>
                        <code className="font-mono text-xs text-slate-400 bg-[var(--govrix-surface-elevated)] px-2 py-0.5 rounded">
                          {rule.conditions}
                        </code>
                      </td>
                      <td>
                        <ActionBadge action={rule.action} />
                      </td>
                      <td className="text-center">
                        <Toggle
                          active={enabled}
                          onChange={() => toggleRule(rule.id, enabled)}
                        />
                      </td>
                    </tr>
                  )
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Policy YAML collapsible */}
      <div className="card overflow-hidden">
        <button
          className="w-full flex items-center justify-between px-5 py-4 hover:bg-[var(--govrix-accent-glow)] transition-colors"
          onClick={() => setYamlOpen(o => !o)}
        >
          <div className="flex items-center gap-2.5">
            <div className="w-2 h-2 rounded-full bg-emerald-400 pulse-glow" />
            <span className="font-display text-sm font-semibold text-[var(--govrix-text-primary)]">
              Policy YAML
            </span>
            <span className="section-label">current config</span>
          </div>
          <div className="flex items-center gap-2 text-[var(--govrix-text-muted)]">
            <span className="text-xs">govrix-policy.yml</span>
            {yamlOpen
              ? <ChevronUp className="w-4 h-4" />
              : <ChevronDown className="w-4 h-4" />}
          </div>
        </button>

        {yamlOpen && (
          <div className="border-t border-[var(--govrix-border)]">
            <div className="bg-[#060a13] p-5 rounded-b-[10px]">
              <YamlViewer yaml={SAMPLE_YAML} />
            </div>
          </div>
        )}
      </div>

    </div>
  )
}
