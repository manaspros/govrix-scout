import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import {
  Clapperboard,
  Search,
  Copy,
  CheckCheck,
  ShieldCheck,
  ShieldAlert,
  Clock,
  Zap,
  Hash,
} from 'lucide-react'
import { clsx } from 'clsx'
import { fetchSession } from '@/api/platform'
import type { SessionRecording, SessionEvent } from '@/api/types'

const SAMPLE_SESSION_ID = 'a3f8c2d1-7e45-4b19-9f3c-8a1d2e6b5c40'

// ── Helpers ───────────────────────────────────────────────────────────────────

function relativeMs(baseIso: string, eventIso: string): string {
  const diffMs = new Date(eventIso).getTime() - new Date(baseIso).getTime()
  if (diffMs === 0) return '+0ms'
  if (diffMs < 1000) return `+${diffMs}ms`
  return `+${(diffMs / 1000).toFixed(1)}s`
}

function formatAbsTime(iso: string): string {
  const d = new Date(iso)
  return d.toLocaleTimeString('en-GB', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' }) +
    `.${String(d.getMilliseconds()).padStart(3, '0')}`
}

// ── Event kind badge config ────────────────────────────────────────────────────

const KIND_STYLES: Record<string, { bg: string; text: string; label: string }> = {
  session_start:  { bg: 'bg-brand-500/10', text: 'text-brand-400',     label: 'SESSION START' },
  session_end:    { bg: 'bg-slate-500/10', text: 'text-slate-400',     label: 'SESSION END' },
  llm_request:    { bg: 'bg-violet-500/10', text: 'text-violet-400',   label: 'LLM REQUEST' },
  llm_response:   { bg: 'bg-blue-500/10',   text: 'text-blue-400',     label: 'LLM RESPONSE' },
  tool_call:      { bg: 'bg-amber-500/10',  text: 'text-amber-400',    label: 'TOOL CALL' },
  tool_result:    { bg: 'bg-orange-500/10', text: 'text-orange-400',   label: 'TOOL RESULT' },
  error:          { bg: 'bg-rose-500/10',   text: 'text-rose-400',     label: 'ERROR' },
  policy_block:   { bg: 'bg-rose-600/10',   text: 'text-rose-500',     label: 'POLICY BLOCK' },
}

function kindStyle(kind: string) {
  return KIND_STYLES[kind] ?? { bg: 'bg-slate-500/10', text: 'text-slate-400', label: kind.toUpperCase() }
}

// ── Copy button ───────────────────────────────────────────────────────────────

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false)

  function handleCopy() {
    void navigator.clipboard.writeText(text).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }

  return (
    <button
      onClick={handleCopy}
      className="ml-2 p-1 rounded text-slate-500 hover:text-slate-300 hover:bg-[var(--govrix-surface-elevated)] transition-colors shrink-0"
      title="Copy to clipboard"
    >
      {copied
        ? <CheckCheck className="w-3.5 h-3.5 text-brand-400" />
        : <Copy className="w-3.5 h-3.5" />}
    </button>
  )
}

// ── Session header card ────────────────────────────────────────────────────────

function SessionHeader({ session }: { session: SessionRecording }) {
  const hashVerified = session.integrity_hash.startsWith('sha256:')

  const durationMs = session.ended_at
    ? new Date(session.ended_at).getTime() - new Date(session.started_at).getTime()
    : null

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`
    return `${(ms / 1000).toFixed(2)}s`
  }

  return (
    <div className="card p-6">
      <div className="flex items-center gap-2 mb-5">
        <div className="w-1.5 h-4 rounded-full bg-brand-500" />
        <h2 className="font-display text-sm font-semibold text-slate-200 tracking-wide">Session Recording</h2>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-5">
        {/* Session ID */}
        <div>
          <div className="section-label mb-1.5">Session ID</div>
          <div className="flex items-center bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border)] rounded-lg px-3 py-2">
            <span className="font-mono text-xs text-slate-300 truncate">{session.session_id}</span>
            <CopyButton text={session.session_id} />
          </div>
        </div>

        {/* Agent ID */}
        <div>
          <div className="section-label mb-1.5">Agent ID</div>
          <div className="flex items-center bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border)] rounded-lg px-3 py-2">
            <span className="font-mono text-xs text-slate-300 truncate">{session.agent_id}</span>
            <CopyButton text={session.agent_id} />
          </div>
        </div>

        {/* Started at */}
        <div>
          <div className="section-label mb-1.5">Started At</div>
          <div className="text-sm text-slate-300 font-mono">
            {new Date(session.started_at).toLocaleString('en-GB', { dateStyle: 'medium', timeStyle: 'long' })}
          </div>
        </div>

        {/* Ended at */}
        <div>
          <div className="section-label mb-1.5">Ended At</div>
          <div className="text-sm text-slate-300 font-mono">
            {session.ended_at
              ? new Date(session.ended_at).toLocaleString('en-GB', { dateStyle: 'medium', timeStyle: 'long' })
              : <span className="text-amber-400">In progress…</span>}
          </div>
        </div>

        {/* Stats row */}
        <div className="flex gap-6">
          <div>
            <div className="section-label mb-1">Event Count</div>
            <div className="stat-value text-xl text-slate-100">{session.event_count}</div>
          </div>
          {durationMs !== null && (
            <div>
              <div className="section-label mb-1">Duration</div>
              <div className="stat-value text-xl text-slate-100">{formatDuration(durationMs)}</div>
            </div>
          )}
        </div>

        {/* Integrity hash */}
        <div className="md:col-span-2">
          <div className="section-label mb-1.5 flex items-center gap-1.5">
            <Hash className="w-3 h-3" />
            Integrity Hash
          </div>
          <div
            className={clsx(
              'flex items-center gap-3 rounded-lg border px-3 py-2.5',
              hashVerified
                ? 'bg-brand-500/[0.04] border-brand-500/20'
                : 'bg-amber-500/[0.04] border-amber-500/20',
            )}
          >
            <span className="font-mono text-xs text-slate-300 break-all flex-1 leading-relaxed">
              {session.integrity_hash}
            </span>
            <div className="shrink-0 flex items-center gap-1.5">
              {hashVerified ? (
                <>
                  <ShieldCheck className="w-4 h-4 text-brand-400" />
                  <span className="text-xs font-semibold text-brand-400 font-display tracking-wide">Verified</span>
                </>
              ) : (
                <>
                  <ShieldAlert className="w-4 h-4 text-amber-400" />
                  <span className="text-xs font-semibold text-amber-400 font-display tracking-wide">Unverified</span>
                </>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

// ── Timeline event item ────────────────────────────────────────────────────────

function TimelineEvent({
  event,
  baseTime,
  isLast,
}: {
  event: SessionEvent
  baseTime: string
  isLast: boolean
}) {
  const isError = event.kind === 'error' || event.kind === 'policy_block' || (event.status_code !== undefined && event.status_code >= 400)
  const style = kindStyle(event.kind)

  return (
    <div className="relative flex gap-4 pl-2">
      {/* Vertical line + dot */}
      <div className="flex flex-col items-center shrink-0">
        <div
          className={clsx(
            'w-2.5 h-2.5 rounded-full border-2 mt-1 shrink-0 z-10',
            isError
              ? 'border-rose-500 bg-rose-500/20'
              : 'border-brand-500 bg-brand-500/20',
          )}
        />
        {!isLast && (
          <div className="w-px flex-1 bg-gradient-to-b from-brand-500/30 to-brand-500/10 mt-1 min-h-[2rem]" />
        )}
      </div>

      {/* Event card */}
      <div
        className={clsx(
          'flex-1 rounded-lg border px-4 py-3 mb-3 transition-colors',
          isError
            ? 'bg-rose-500/[0.04] border-rose-500/20'
            : 'bg-[var(--govrix-surface-elevated)] border-[var(--govrix-border)] hover:border-[var(--govrix-border-strong)]',
        )}
      >
        <div className="flex flex-wrap items-center gap-2 mb-2">
          {/* Relative time */}
          <span className="font-mono text-xs text-brand-400 font-medium">
            {relativeMs(baseTime, event.timestamp)}
          </span>

          {/* Kind badge */}
          <span
            className={clsx(
              'px-2 py-0.5 rounded-md text-[10px] font-semibold font-display tracking-wider border',
              style.bg,
              style.text,
              isError ? 'border-rose-500/20' : 'border-transparent',
            )}
          >
            {style.label}
          </span>

          {/* Absolute timestamp */}
          <span className="font-mono text-[11px] text-slate-600 ml-auto">
            {formatAbsTime(event.timestamp)}
          </span>
        </div>

        {/* Metrics row */}
        <div className="flex flex-wrap gap-4 text-xs">
          {event.model && (
            <div className="flex items-center gap-1 text-slate-500">
              <span className="text-slate-600">model</span>
              <span className="font-mono text-slate-400">{event.model}</span>
            </div>
          )}

          {(event.input_tokens !== undefined || event.output_tokens !== undefined) && (
            <div className="flex items-center gap-1 text-slate-500">
              <span className="text-slate-600">tokens</span>
              <span className="font-mono text-slate-400">
                {event.input_tokens?.toLocaleString() ?? '—'}
                <span className="text-slate-600 mx-0.5">in</span>
                /
                <span className="mx-0.5" />
                {event.output_tokens?.toLocaleString() ?? '—'}
                <span className="text-slate-600 ml-0.5">out</span>
              </span>
            </div>
          )}

          {event.latency_ms !== undefined && (
            <div className="flex items-center gap-1 text-slate-500">
              <Clock className="w-3 h-3 text-slate-600" />
              <span className="font-mono text-slate-400">{event.latency_ms}ms</span>
            </div>
          )}

          {event.status_code !== undefined && (
            <div className="flex items-center gap-1">
              <span
                className={clsx(
                  'font-mono text-xs font-semibold',
                  event.status_code >= 500
                    ? 'text-rose-400'
                    : event.status_code >= 400
                    ? 'text-amber-400'
                    : 'text-brand-400',
                )}
              >
                HTTP {event.status_code}
              </span>
            </div>
          )}

          {event.error_message && (
            <div className="w-full mt-1 font-mono text-[11px] text-rose-400 bg-rose-500/5 rounded px-2 py-1">
              {event.error_message}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// ── Empty state ────────────────────────────────────────────────────────────────

function SessionEmptyState() {
  return (
    <div className="card flex flex-col items-center justify-center py-20 gap-4">
      <div className="w-14 h-14 rounded-2xl bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border)] flex items-center justify-center">
        <Clapperboard className="w-6 h-6 text-slate-600" />
      </div>
      <div className="text-center">
        <p className="text-slate-400 text-sm font-medium">Enter a session ID to view the integrity-sealed recording</p>
        <p className="text-slate-600 text-xs mt-1">SHA-256 sealed • Tamper-evident • Forensic-grade audit trail</p>
      </div>
    </div>
  )
}

// ── Skeleton loader ────────────────────────────────────────────────────────────

function SessionSkeleton() {
  return (
    <div className="space-y-4">
      <div className="card p-6 space-y-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <div key={i} className="skeleton h-10 rounded-lg w-full" />
        ))}
      </div>
      <div className="card p-6 space-y-3">
        {Array.from({ length: 5 }).map((_, i) => (
          <div key={i} className="skeleton h-16 rounded-lg w-full" />
        ))}
      </div>
    </div>
  )
}

// ── Main page ─────────────────────────────────────────────────────────────────

export function SessionsPage() {
  const [inputValue, setInputValue] = useState('')
  const [searchId, setSearchId] = useState<string | null>(null)

  const { data, isLoading, error } = useQuery({
    queryKey: ['session', searchId],
    queryFn: () => fetchSession(searchId!),
    enabled: searchId !== null,
    retry: 1,
    staleTime: 30_000,
  })

  function handleSearch() {
    const trimmed = inputValue.trim()
    if (!trimmed) return
    setSearchId(trimmed)
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === 'Enter') handleSearch()
  }

  function handleUseSample() {
    setInputValue(SAMPLE_SESSION_ID)
    setSearchId(SAMPLE_SESSION_ID)
  }

  const displaySession = data ?? null

  return (
    <div className="space-y-5 page-enter">
      {/* Page header */}
      <div>
        <h1 className="font-display text-xl font-semibold text-slate-100 tracking-tight">Session Forensics</h1>
        <p className="text-sm text-slate-500 mt-0.5">
          Integrity-sealed session replay — every event cryptographically verified
        </p>
      </div>

      {/* Search bar */}
      <div className="flex gap-2.5">
        <div className="relative flex-1">
          <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500 pointer-events-none" />
          <input
            type="text"
            placeholder="Enter session ID (UUID)..."
            value={inputValue}
            onChange={e => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            className="w-full pl-10 pr-4 py-2.5 text-sm font-mono text-slate-200 placeholder:text-slate-600 bg-[var(--govrix-surface-elevated)] border border-[var(--govrix-border)] rounded-lg focus:outline-none focus:ring-1 focus:ring-brand-500 focus:border-brand-500/50 transition-colors"
          />
        </div>
        <button onClick={handleSearch} className="btn btn-primary px-5">
          <Search className="w-4 h-4" />
          Search
        </button>
        {searchId !== null && (
          <button
            onClick={() => {
              setSearchId(null)
              setInputValue('')
            }}
            className="btn btn-ghost px-4"
          >
            Clear
          </button>
        )}
      </div>

      {/* Sample hint — shown when no search is active */}
      {searchId === null && (
        <div className="flex items-center gap-2">
          <div className="h-px flex-1 bg-[var(--govrix-border)]" />
          <span className="section-label flex items-center gap-1.5 whitespace-nowrap">
            <Zap className="w-3 h-3 text-amber-500" />
            Try sample ID:
            <button
              onClick={handleUseSample}
              className="font-mono text-brand-400 hover:text-brand-300 underline underline-offset-2 text-[0.6875rem] transition-colors"
            >
              {SAMPLE_SESSION_ID}
            </button>
          </span>
          <div className="h-px flex-1 bg-[var(--govrix-border)]" />
        </div>
      )}

      {/* Content */}
      {isLoading && searchId !== null ? (
        <SessionSkeleton />
      ) : error && searchId !== null ? (
        <div className="card p-8 flex flex-col items-center gap-3">
          <ShieldAlert className="w-8 h-8 text-rose-400" />
          <p className="text-rose-400 text-sm font-medium">Session not found or access denied</p>
          <p className="text-slate-600 text-xs font-mono">{searchId}</p>
        </div>
      ) : displaySession ? (
        <div className="space-y-5 stagger-in">
          {/* Header card */}
          <SessionHeader session={displaySession} />

          {/* Timeline */}
          <div className="card p-6">
            <div className="flex items-center gap-2 mb-6">
              <div className="w-1.5 h-4 rounded-full bg-brand-500" />
              <h2 className="font-display text-sm font-semibold text-slate-200 tracking-wide">Event Timeline</h2>
              <span className="ml-auto text-xs text-slate-600 font-mono">
                {displaySession.events.length} events
              </span>
            </div>

            <div className="space-y-0">
              {displaySession.events.map((event, i) => (
                <TimelineEvent
                  key={i}
                  event={event}
                  baseTime={displaySession.started_at}
                  isLast={i === displaySession.events.length - 1}
                />
              ))}
            </div>
          </div>
        </div>
      ) : (
        <SessionEmptyState />
      )}
    </div>
  )
}
