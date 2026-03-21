import { useState } from 'react'
import {
  AlertTriangle,
  Zap,
  Users,
  XCircle,
  CircuitBoard,
  CheckCircle,
  Loader2,
  SkullIcon,
  Clock,
  Heart,
  Target,
  ShieldOff,
} from 'lucide-react'
import { clsx } from 'clsx'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useAgents } from '@/api/hooks'
import { fetchKillSwitchHistory, killAgent, reviveAgent, fetchKillSwitchStatus } from '@/api/platform'
import type { Agent, KillEvent } from '@/api/types'
import { EnterpriseFeatureCard, isNotFoundError } from '@/components/common/EnterpriseFeatureCard'

// ── Types ─────────────────────────────────────────────────────────────────────

type KillState = 'idle' | 'killing' | 'killed'

type KillMethod = 'manual' | 'circuit_breaker' | 'budget'

// ── Kill method badge ──────────────────────────────────────────────────────────

const METHOD_STYLES: Record<KillMethod, { bg: string; text: string; border: string; label: string }> = {
  manual:          { bg: 'bg-slate-500/10', text: 'text-slate-400',  border: 'border-slate-500/20', label: 'Manual' },
  circuit_breaker: { bg: 'bg-amber-500/10', text: 'text-amber-400',  border: 'border-amber-500/20', label: 'Circuit Breaker' },
  budget:          { bg: 'bg-violet-500/10', text: 'text-violet-400', border: 'border-violet-500/20', label: 'Budget' },
}

function MethodBadge({ method }: { method: KillMethod }) {
  const s = METHOD_STYLES[method] ?? METHOD_STYLES['manual']
  return (
    <span
      className={clsx(
        'inline-flex items-center px-2 py-0.5 rounded-md text-[10px] font-semibold font-display tracking-wider border',
        s.bg, s.text, s.border,
      )}
    >
      {s.label}
    </span>
  )
}

// ── Status badge ───────────────────────────────────────────────────────────────

const STATUS_CFG: Record<string, { dot: string; text: string; label: string; pulse: boolean }> = {
  active:  { dot: 'bg-brand-400', text: 'text-brand-400', label: 'Active',  pulse: true  },
  idle:    { dot: 'bg-slate-500', text: 'text-slate-400', label: 'Idle',    pulse: false },
  error:   { dot: 'bg-rose-500',  text: 'text-rose-400',  label: 'Error',   pulse: false },
  blocked: { dot: 'bg-amber-500', text: 'text-amber-400', label: 'Blocked', pulse: false },
  retired: { dot: 'bg-slate-600', text: 'text-slate-500', label: 'Retired', pulse: false },
}

function AgentStatusBadge({ status }: { status: string }) {
  const cfg = STATUS_CFG[status] ?? STATUS_CFG['idle']
  return (
    <div className="flex items-center gap-1.5">
      <span
        className={clsx('w-2 h-2 rounded-full shrink-0', cfg.dot, cfg.pulse && 'pulse-glow')}
      />
      <span className={clsx('text-xs font-medium', cfg.text)}>{cfg.label}</span>
    </div>
  )
}

// ── Stat card ─────────────────────────────────────────────────────────────────

interface StatCardProps {
  label: string
  value: number | string
  icon: React.ReactNode
  accent: 'green' | 'rose' | 'amber'
}

function StatCard({ label, value, icon, accent }: StatCardProps) {
  const accentClasses = {
    green: { icon: 'text-brand-400 bg-brand-500/10', value: 'text-brand-400' },
    rose:  { icon: 'text-rose-400 bg-rose-500/10',   value: 'text-rose-400' },
    amber: { icon: 'text-amber-400 bg-amber-500/10', value: 'text-amber-400' },
  }
  const cls = accentClasses[accent]

  return (
    <div className="card p-5 flex items-start gap-4">
      <div className={clsx('w-10 h-10 rounded-lg flex items-center justify-center shrink-0', cls.icon)}>
        {icon}
      </div>
      <div>
        <div className="section-label mb-1">{label}</div>
        <div className={clsx('stat-value text-2xl', cls.value)}>{value}</div>
      </div>
    </div>
  )
}

// ── Kill button with states ────────────────────────────────────────────────────

function KillButton({ agentId }: { agentId: string }) {
  const [state, setState] = useState<KillState>('idle')

  function handleKill() {
    if (state !== 'idle') return
    setState('killing')
    // Simulate async kill operation
    setTimeout(() => setState('killed'), 1400)
  }

  if (state === 'idle') {
    return (
      <button
        onClick={handleKill}
        className="btn btn-danger text-xs px-3 py-1.5"
        title={`Kill agent ${agentId}`}
      >
        <XCircle className="w-3.5 h-3.5" />
        Kill
      </button>
    )
  }

  if (state === 'killing') {
    return (
      <button disabled className="btn btn-danger text-xs px-3 py-1.5 opacity-70 cursor-not-allowed">
        <Loader2 className="w-3.5 h-3.5 animate-spin" />
        Killing…
      </button>
    )
  }

  return (
    <button disabled className="btn text-xs px-3 py-1.5 bg-rose-900/20 text-rose-500 border border-rose-900/30 cursor-not-allowed">
      <CheckCircle className="w-3.5 h-3.5" />
      Killed
    </button>
  )
}

// ── Emergency banner ──────────────────────────────────────────────────────────

function EmergencyBanner({ activeCount }: { activeCount: number }) {
  const [showConfirm, setShowConfirm] = useState(false)
  const [confirmText, setConfirmText] = useState('')
  const [killAllState, setKillAllState] = useState<'idle' | 'executing' | 'done'>('idle')

  function handleKillAll() {
    if (confirmText !== 'CONFIRM') return
    setKillAllState('executing')
    setTimeout(() => setKillAllState('done'), 2000)
  }

  return (
    <div className="rounded-xl border border-rose-500/20 bg-rose-500/[0.04] p-5">
      <div className="flex flex-col sm:flex-row sm:items-center gap-4">
        <div className="flex items-start gap-3 flex-1 min-w-0">
          <div className="w-10 h-10 rounded-lg bg-rose-500/10 border border-rose-500/20 flex items-center justify-center shrink-0 mt-0.5">
            <AlertTriangle className="w-5 h-5 text-rose-400" />
          </div>
          <div>
            <h3 className="font-display text-sm font-semibold text-rose-300">Emergency Kill Switch</h3>
            <p className="text-xs text-slate-500 mt-0.5">
              Immediately terminates all active agent sessions — this action cannot be undone
            </p>
            {activeCount > 0 && (
              <p className="text-xs text-rose-400/70 mt-1">
                {activeCount} active agent{activeCount !== 1 ? 's' : ''} will be terminated
              </p>
            )}
          </div>
        </div>

        {killAllState === 'done' ? (
          <div className="flex items-center gap-2 text-sm font-medium text-rose-400">
            <CheckCircle className="w-5 h-5" />
            All agents terminated
          </div>
        ) : !showConfirm ? (
          <button
            onClick={() => setShowConfirm(true)}
            className="btn btn-danger px-5 py-2.5 text-sm shrink-0 border border-rose-500/30"
            disabled={activeCount === 0}
          >
            <Zap className="w-4 h-4" />
            Kill All Agents
          </button>
        ) : (
          <div className="flex items-center gap-2.5 shrink-0">
            <input
              autoFocus
              type="text"
              value={confirmText}
              onChange={e => setConfirmText(e.target.value.toUpperCase())}
              placeholder="Type CONFIRM"
              className="w-36 px-3 py-2 text-sm font-mono text-rose-300 placeholder:text-rose-900 bg-rose-500/5 border border-rose-500/30 rounded-lg focus:outline-none focus:ring-1 focus:ring-rose-500 tracking-widest"
            />
            <button
              onClick={handleKillAll}
              disabled={confirmText !== 'CONFIRM' || killAllState === 'executing'}
              className={clsx(
                'btn btn-danger px-4 py-2 text-sm',
                (confirmText !== 'CONFIRM' || killAllState === 'executing') && 'opacity-40 cursor-not-allowed',
              )}
            >
              {killAllState === 'executing'
                ? <><Loader2 className="w-4 h-4 animate-spin" /> Executing…</>
                : <><Zap className="w-4 h-4" /> Confirm Kill</>}
            </button>
            <button
              onClick={() => { setShowConfirm(false); setConfirmText('') }}
              className="btn btn-ghost px-3 py-2 text-sm"
            >
              Cancel
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

// ── Relative time helper ──────────────────────────────────────────────────────

function relativeTime(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime()
  const diffMin = Math.floor(diffMs / 60_000)
  if (diffMin < 1) return 'just now'
  if (diffMin < 60) return `${diffMin}m ago`
  const diffHr = Math.floor(diffMin / 60)
  if (diffHr < 24) return `${diffHr}h ago`
  return `${Math.floor(diffHr / 24)}d ago`
}

// ── Kill history table ────────────────────────────────────────────────────────

function KillHistorySection({ events, isLoading }: { events: KillEvent[]; isLoading: boolean }) {
  return (
    <div className="card overflow-hidden">
      <div className="px-5 pt-5 pb-4 border-b border-[var(--govrix-border)]">
        <div className="section-label flex items-center gap-1.5">
          <Clock className="w-3 h-3" />
          Recent Kill Events
        </div>
      </div>
      {isLoading ? (
        <div className="p-5 space-y-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="skeleton h-10 rounded-lg" />
          ))}
        </div>
      ) : events.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 gap-3">
          <CheckCircle className="w-8 h-8 text-brand-400/50" />
          <p className="text-slate-500 text-sm">No kill events recorded yet</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="govrix-table">
            <thead>
              <tr>
                <th>Time</th>
                <th>Agent</th>
                <th>Killed By</th>
                <th>Reason</th>
                <th>Method</th>
              </tr>
            </thead>
            <tbody>
              {events.map((entry, idx) => (
                <tr key={`${entry.agent_id}-${idx}`}>
                  <td className="whitespace-nowrap">
                    <span className="font-mono text-slate-500 text-xs">{relativeTime(entry.time)}</span>
                  </td>
                  <td>
                    <div className="flex flex-col gap-0.5">
                      <span className="text-slate-300 text-xs font-medium">{entry.agent_name}</span>
                      <span className="font-mono text-slate-600 text-[11px]">{entry.agent_id.slice(0, 20)}…</span>
                    </div>
                  </td>
                  <td>
                    <span className={clsx('font-mono text-xs', entry.killed_by === 'system' ? 'text-amber-400/70' : 'text-slate-400')}>
                      {entry.killed_by}
                    </span>
                  </td>
                  <td className="max-w-[280px]">
                    <span className="text-slate-400 text-xs leading-relaxed">{entry.reason}</span>
                  </td>
                  <td>
                    <MethodBadge method={entry.method} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

// ── Agent control table ────────────────────────────────────────────────────────

function AgentControlTable({ agents }: { agents: Agent[] }) {
  if (agents.length === 0) {
    return (
      <div className="card flex flex-col items-center justify-center py-16 gap-3">
        <SkullIcon className="w-8 h-8 text-slate-700" />
        <p className="text-slate-500 text-sm">No agents registered</p>
      </div>
    )
  }

  return (
    <div className="card overflow-hidden">
      <div className="px-5 pt-5 pb-4 border-b border-[var(--govrix-border)] flex items-center justify-between">
        <div className="section-label flex items-center gap-1.5">
          <Users className="w-3 h-3" />
          Agent Control Panel
        </div>
        <span className="text-xs text-slate-600 font-mono">{agents.length} agents registered</span>
      </div>
      <div className="overflow-x-auto">
        <table className="govrix-table">
          <thead>
            <tr>
              <th>Agent ID</th>
              <th>Name</th>
              <th>Status</th>
              <th className="text-right">Requests (24h)</th>
              <th className="text-right">Tokens (24h)</th>
              <th>Actions</th>
            </tr>
          </thead>
          <tbody>
            {agents.map(agent => (
              <tr key={agent.id}>
                <td>
                  <span className="font-mono text-xs text-slate-500">
                    {agent.id.slice(0, 8)}…{agent.id.slice(-4)}
                  </span>
                </td>
                <td>
                  <span className="text-slate-300 text-sm font-medium">{agent.name ?? 'Unnamed'}</span>
                </td>
                <td>
                  <AgentStatusBadge status={agent.status} />
                </td>
                <td className="text-right">
                  <span className="font-mono text-xs text-slate-400">
                    {(agent.total_requests ?? 0).toLocaleString()}
                  </span>
                </td>
                <td className="text-right">
                  <span className="font-mono text-xs text-slate-400">
                    {(agent.total_tokens ?? 0).toLocaleString()}
                  </span>
                </td>
                <td>
                  <KillButton agentId={agent.id} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

// ── Kill Agent Form ──────────────────────────────────────────────────────────

function KillAgentForm() {
  const queryClient = useQueryClient()
  const [killAgentId, setKillAgentId] = useState('')
  const [killReason, setKillReason] = useState('')
  const [showKillConfirm, setShowKillConfirm] = useState(false)
  const [confirmText, setConfirmText] = useState('')

  const killMutation = useMutation({
    mutationFn: () => killAgent(killAgentId, killReason),
    onSuccess: () => {
      setKillAgentId('')
      setKillReason('')
      setShowKillConfirm(false)
      setConfirmText('')
      queryClient.invalidateQueries({ queryKey: ['kill-switch-history'] })
      queryClient.invalidateQueries({ queryKey: ['kill-switch-status'] })
    },
  })

  function handleInitiateKill() {
    if (!killAgentId.trim() || !killReason.trim()) return
    setShowKillConfirm(true)
  }

  function handleConfirmKill() {
    if (confirmText !== 'CONFIRM') return
    killMutation.mutate()
  }

  function handleCancelKill() {
    setShowKillConfirm(false)
    setConfirmText('')
  }

  return (
    <div className="rounded-xl border border-red-500/20 bg-red-500/[0.04] p-5">
      <div className="flex items-start gap-3 mb-4">
        <div className="w-10 h-10 rounded-lg bg-red-500/10 border border-red-500/20 flex items-center justify-center shrink-0 mt-0.5">
          <Target className="w-5 h-5 text-red-400" />
        </div>
        <div>
          <h3 className="font-display text-sm font-semibold text-red-300">Kill Agent</h3>
          <p className="text-xs text-slate-500 mt-0.5">
            Terminate a specific agent session by ID
          </p>
        </div>
      </div>

      {!showKillConfirm ? (
        <div className="flex flex-col sm:flex-row gap-3">
          <input
            type="text"
            value={killAgentId}
            onChange={e => setKillAgentId(e.target.value)}
            placeholder="Agent ID (e.g. agt_prod_4e5f6a7b...)"
            className="flex-1 px-3 py-2 text-sm font-mono text-slate-300 placeholder:text-slate-600 bg-[var(--govrix-surface)] border border-[var(--govrix-border)] rounded-lg focus:outline-none focus:ring-1 focus:ring-red-500/50"
          />
          <input
            type="text"
            value={killReason}
            onChange={e => setKillReason(e.target.value)}
            placeholder="Reason for termination"
            className="flex-1 px-3 py-2 text-sm text-slate-300 placeholder:text-slate-600 bg-[var(--govrix-surface)] border border-[var(--govrix-border)] rounded-lg focus:outline-none focus:ring-1 focus:ring-red-500/50"
          />
          <button
            onClick={handleInitiateKill}
            disabled={!killAgentId.trim() || !killReason.trim()}
            className={clsx(
              'btn btn-danger px-5 py-2 text-sm shrink-0 border border-red-500/30',
              (!killAgentId.trim() || !killReason.trim()) && 'opacity-40 cursor-not-allowed',
            )}
          >
            <XCircle className="w-4 h-4" />
            Kill Agent
          </button>
        </div>
      ) : (
        <div className="space-y-3">
          <div className="rounded-lg border border-red-500/30 bg-red-500/[0.06] p-3">
            <p className="text-sm text-red-300">
              Are you sure you want to terminate agent{' '}
              <span className="font-mono font-semibold">{killAgentId}</span>?
            </p>
            <p className="text-xs text-slate-500 mt-1">
              Reason: {killReason}
            </p>
          </div>
          <div className="flex items-center gap-2.5">
            <input
              autoFocus
              type="text"
              value={confirmText}
              onChange={e => setConfirmText(e.target.value.toUpperCase())}
              placeholder="Type CONFIRM to proceed"
              className="w-52 px-3 py-2 text-sm font-mono text-red-300 placeholder:text-red-900 bg-red-500/5 border border-red-500/30 rounded-lg focus:outline-none focus:ring-1 focus:ring-red-500 tracking-widest"
            />
            <button
              onClick={handleConfirmKill}
              disabled={confirmText !== 'CONFIRM' || killMutation.isPending}
              className={clsx(
                'btn btn-danger px-4 py-2 text-sm',
                (confirmText !== 'CONFIRM' || killMutation.isPending) && 'opacity-40 cursor-not-allowed',
              )}
            >
              {killMutation.isPending
                ? <><Loader2 className="w-4 h-4 animate-spin" /> Killing…</>
                : <><Zap className="w-4 h-4" /> Confirm Kill</>}
            </button>
            <button
              onClick={handleCancelKill}
              className="btn btn-ghost px-3 py-2 text-sm"
            >
              Cancel
            </button>
          </div>
          {killMutation.isError && (
            <p className="text-xs text-red-400">
              Failed to kill agent: {killMutation.error?.message ?? 'Unknown error'}
            </p>
          )}
        </div>
      )}

      {killMutation.isSuccess && (
        <div className="flex items-center gap-2 mt-3 text-sm font-medium text-red-400">
          <CheckCircle className="w-4 h-4" />
          Agent terminated successfully
        </div>
      )}
    </div>
  )
}

// ── Currently Killed Agents ──────────────────────────────────────────────────

function KilledAgentsSection() {
  const queryClient = useQueryClient()

  const { data: statusData, isLoading, error: statusError } = useQuery({
    queryKey: ['kill-switch-status'],
    queryFn: fetchKillSwitchStatus,
    // Only poll if the endpoint is available (not 404)
    refetchInterval: (query) => {
      if (query.state.error && isNotFoundError(query.state.error)) return false
      return 10000
    },
    retry: 1,
  })

  const reviveMutation = useMutation({
    mutationFn: (agentId: string) => reviveAgent(agentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['kill-switch-status'] })
      queryClient.invalidateQueries({ queryKey: ['kill-switch-history'] })
    },
  })

  // If the status endpoint returned a 404, don't show this section at all
  if (statusError && isNotFoundError(statusError)) {
    return null
  }

  const killedAgents = statusData?.killed_agents ?? []
  const recentKills = statusData?.recent_kills ?? []

  return (
    <div className="card overflow-hidden">
      <div className="px-5 pt-5 pb-4 border-b border-[var(--govrix-border)] flex items-center justify-between">
        <div className="section-label flex items-center gap-1.5">
          <ShieldOff className="w-3 h-3" />
          Currently Killed Agents
        </div>
        <span className="text-xs text-slate-600 font-mono">
          {isLoading ? '...' : `${killedAgents.length} terminated`}
        </span>
      </div>

      {isLoading ? (
        <div className="p-5 space-y-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="skeleton h-10 rounded-lg" />
          ))}
        </div>
      ) : killedAgents.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 gap-3">
          <CheckCircle className="w-8 h-8 text-brand-400/50" />
          <p className="text-slate-500 text-sm">No agents currently terminated</p>
        </div>
      ) : (
        <div className="divide-y divide-[var(--govrix-border)]">
          {killedAgents.map(agentId => {
            const killInfo = recentKills.find(k => k.agent_id === agentId)
            return (
              <div key={agentId} className="px-5 py-3 flex items-center justify-between gap-4">
                <div className="flex items-center gap-3 min-w-0">
                  <span className="w-2 h-2 rounded-full bg-red-500 shrink-0" />
                  <div className="min-w-0">
                    <span className="font-mono text-xs text-slate-300 block truncate">
                      {agentId}
                    </span>
                    {killInfo && (
                      <span className="text-[11px] text-slate-600 block truncate">
                        {killInfo.reason} &mdash; {killInfo.killed_by}
                      </span>
                    )}
                  </div>
                </div>
                <button
                  onClick={() => reviveMutation.mutate(agentId)}
                  disabled={reviveMutation.isPending && reviveMutation.variables === agentId}
                  className={clsx(
                    'inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-semibold rounded-lg border transition-colors',
                    'text-emerald-400 bg-emerald-500/10 border-emerald-500/20 hover:bg-emerald-500/20',
                    reviveMutation.isPending && reviveMutation.variables === agentId && 'opacity-50 cursor-not-allowed',
                  )}
                >
                  {reviveMutation.isPending && reviveMutation.variables === agentId ? (
                    <><Loader2 className="w-3.5 h-3.5 animate-spin" /> Reviving…</>
                  ) : (
                    <><Heart className="w-3.5 h-3.5" /> Revive</>
                  )}
                </button>
              </div>
            )
          })}
        </div>
      )}

      {reviveMutation.isError && (
        <div className="px-5 pb-3">
          <p className="text-xs text-red-400">
            Failed to revive agent: {reviveMutation.error?.message ?? 'Unknown error'}
          </p>
        </div>
      )}
    </div>
  )
}

// ── Main page ─────────────────────────────────────────────────────────────────

export function KillSwitchPage() {
  const { data: agentsData, isLoading: agentsLoading } = useAgents()
  const agents = agentsData?.agents ?? []

  const { data: killHistoryData, isLoading: historyLoading, error: historyError } = useQuery({
    queryKey: ['kill-switch-history'],
    queryFn: fetchKillSwitchHistory,
    staleTime: 30_000,
    retry: 1,
  })

  // If the kill-switch history API returned a 404, this is an enterprise-only feature
  if (!historyLoading && historyError && isNotFoundError(historyError)) {
    return (
      <div className="space-y-5 page-enter">
        <div>
          <h1 className="font-display text-xl font-semibold text-slate-100 tracking-tight flex items-center gap-2">
            <SkullIcon className="w-5 h-5 text-rose-400" />
            Kill Switch
          </h1>
          <p className="text-sm text-slate-500 mt-0.5">
            Agent control and emergency operations
          </p>
        </div>
        <div className="card">
          <EnterpriseFeatureCard
            icon={SkullIcon}
            title="Kill Switch requires Govrix Enterprise"
            description="The kill switch provides one-click agent session termination, circuit breaker automation, and emergency kill-all capabilities. It includes real-time status monitoring and a full audit trail of kill events. Upgrade to Govrix Enterprise to enable this feature."
          />
        </div>
      </div>
    )
  }

  const killEvents: KillEvent[] = killHistoryData?.events ?? []

  const activeCount = agents.filter(a => a.status === 'active').length
  const killedToday = killHistoryData?.killed_today ?? 0
  const circuitBreakers = killHistoryData?.circuit_breakers_triggered ?? 0

  return (
    <div className="space-y-5 page-enter">
      {/* Page header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="font-display text-xl font-semibold text-slate-100 tracking-tight flex items-center gap-2">
            <SkullIcon className="w-5 h-5 text-rose-400" />
            Kill Switch
          </h1>
          <p className="text-sm text-slate-500 mt-0.5">
            Agent control and emergency operations — actions here are immediate and irreversible
          </p>
        </div>
      </div>

      {/* Kill Agent form */}
      <KillAgentForm />

      {/* Emergency banner */}
      <EmergencyBanner activeCount={activeCount} />

      {/* Stats row */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 stagger-in">
        <StatCard
          label="Active Agents"
          value={agentsLoading ? '—' : activeCount}
          icon={<Users className="w-5 h-5" />}
          accent="green"
        />
        <StatCard
          label="Killed Today"
          value={historyLoading ? '—' : killedToday}
          icon={<XCircle className="w-5 h-5" />}
          accent="rose"
        />
        <StatCard
          label="Circuit Breakers Triggered"
          value={historyLoading ? '—' : circuitBreakers}
          icon={<CircuitBoard className="w-5 h-5" />}
          accent="amber"
        />
      </div>

      {/* Currently killed agents */}
      <KilledAgentsSection />

      {/* Agent control table */}
      {agentsLoading ? (
        <div className="card overflow-hidden">
          <div className="px-5 pt-5 pb-4 border-b border-[var(--govrix-border)]">
            <div className="skeleton h-4 w-40 rounded" />
          </div>
          <div className="p-5 space-y-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <div key={i} className="skeleton h-12 rounded-lg" />
            ))}
          </div>
        </div>
      ) : (
        <AgentControlTable agents={agents} />
      )}

      {/* Kill history */}
      <KillHistorySection events={killEvents} isLoading={historyLoading} />
    </div>
  )
}
