import { useState, useEffect, useRef, useCallback } from 'react'
import { formatDistanceToNow, parseISO } from 'date-fns'
import { ChevronDown, ChevronUp, Activity, Wifi, WifiOff } from 'lucide-react'
import { clsx } from 'clsx'

// ── Types ─────────────────────────────────────────────────────────────────────

interface StreamEvent {
  id: string
  timestamp: string
  event_kind: string
  agent_id: string
  tool_name?: string
  model?: string
  risk_score?: number
  latency_ms?: number
  session_id?: string
}

// ── Kind badge config ─────────────────────────────────────────────────────────

const KIND_COLORS: Record<string, string> = {
  'llm.request':    '#3b82f6',
  'llm.response':   '#60a5fa',
  'tool.invoke':    '#f97316',
  'tool.result':    '#22c55e',
  'policy.block':   '#ef4444',
  'agent.spawn':    '#a855f7',
  'agent.message':  '#8b5cf6',
  'session.start':  '#10b981',
  'session.end':    '#6b7280',
}

function kindColor(kind: string): string {
  return KIND_COLORS[kind] ?? '#64748b'
}

function KindBadge({ kind }: { kind: string }) {
  const color = kindColor(kind)
  return (
    <span
      className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-semibold uppercase tracking-wider shrink-0"
      style={{
        color,
        background: `${color}18`,
        border: `1px solid ${color}33`,
        fontFamily: 'JetBrains Mono',
      }}
    >
      {kind.replace('.', ' ')}
    </span>
  )
}

// ── Mini risk bar ─────────────────────────────────────────────────────────────

function MiniRiskBar({ score }: { score: number }) {
  const pct = Math.min(100, Math.max(0, score))
  const color = pct < 30 ? '#22c55e' : pct < 70 ? '#eab308' : '#ef4444'
  return (
    <div className="flex items-center gap-1.5">
      <div className="w-12 h-1 bg-slate-700 rounded-full overflow-hidden">
        <div className="h-full rounded-full" style={{ width: `${pct}%`, background: color }} />
      </div>
      <span className="text-[10px] tabular-nums" style={{ color, fontFamily: 'JetBrains Mono' }}>
        {pct}
      </span>
    </div>
  )
}

// ── SSE connection hook ───────────────────────────────────────────────────────

function getApiKey(): string {
  return (
    (import.meta.env.VITE_API_KEY as string | undefined) ||
    localStorage.getItem('govrix_api_key') ||
    'govrix-local-dev'
  )
}

const MAX_EVENTS = 50

function useEventStream() {
  const [events, setEvents] = useState<StreamEvent[]>([])
  const [connected, setConnected] = useState(false)
  const esRef = useRef<EventSource | null>(null)
  const reconnectRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const connect = useCallback(() => {
    if (esRef.current) {
      esRef.current.close()
    }

    const apiBase = import.meta.env.VITE_API_URL ?? ''
    // EventSource doesn't support custom headers natively; use query param fallback
    const url = `${apiBase}/api/v1/events/stream?api_key=${getApiKey()}`

    try {
      const es = new EventSource(url)
      esRef.current = es

      es.onopen = () => setConnected(true)

      es.onmessage = (e: MessageEvent) => {
        try {
          const data = JSON.parse(e.data as string) as StreamEvent
          setEvents(prev => {
            const updated = [data, ...prev]
            return updated.slice(0, MAX_EVENTS)
          })
        } catch {
          // ignore parse errors
        }
      }

      es.onerror = () => {
        setConnected(false)
        es.close()
        // Reconnect after 5s
        reconnectRef.current = setTimeout(connect, 5000)
      }
    } catch {
      setConnected(false)
    }
  }, [])

  useEffect(() => {
    connect()
    return () => {
      esRef.current?.close()
      if (reconnectRef.current) clearTimeout(reconnectRef.current)
    }
  }, [connect])

  return { events, connected }
}

// ── Event row ─────────────────────────────────────────────────────────────────

function EventStreamRow({ event }: { event: StreamEvent }) {
  const relTime = (() => {
    try {
      return formatDistanceToNow(parseISO(event.timestamp), { addSuffix: true, includeSeconds: true })
    } catch {
      return '—'
    }
  })()

  return (
    <div
      className="flex items-center gap-3 px-4 py-2 border-b hover:bg-white/[0.015] transition-colors"
      style={{ borderColor: 'rgba(148,163,184,0.05)' }}
    >
      {/* Time */}
      <span
        className="text-[10px] text-slate-600 whitespace-nowrap shrink-0 w-16"
        style={{ fontFamily: 'JetBrains Mono' }}
      >
        {relTime.replace(' ago', '').replace('less than a', '<1')}
      </span>

      {/* Kind */}
      <div className="shrink-0">
        <KindBadge kind={event.event_kind} />
      </div>

      {/* Agent */}
      <span className="text-xs text-slate-400 truncate max-w-[100px] shrink-0">
        {event.agent_id}
      </span>

      {/* Tool / Model */}
      <span className="text-xs text-slate-500 truncate flex-1 min-w-0">
        {event.tool_name ?? event.model ?? '—'}
      </span>

      {/* Risk */}
      {event.risk_score != null && event.risk_score > 0 ? (
        <div className="shrink-0">
          <MiniRiskBar score={event.risk_score} />
        </div>
      ) : (
        <div className="w-20 shrink-0" />
      )}

      {/* Latency */}
      <span
        className="text-[10px] text-slate-600 w-14 text-right shrink-0"
        style={{ fontFamily: 'JetBrains Mono' }}
      >
        {event.latency_ms != null ? `${event.latency_ms}ms` : ''}
      </span>
    </div>
  )
}

// ── EventStream component ─────────────────────────────────────────────────────

interface EventStreamProps {
  /** If true, render as a full page (no collapse toggle) */
  fullPage?: boolean
  /** Max height for the scroll container */
  maxHeight?: number
}

export function EventStream({ fullPage = false, maxHeight = 400 }: EventStreamProps) {
  const { events, connected } = useEventStream()
  const [collapsed, setCollapsed] = useState(false)
  const [userScrolled, setUserScrolled] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)

  // Auto-scroll to top (newest is first in the list)
  useEffect(() => {
    if (!userScrolled && containerRef.current) {
      containerRef.current.scrollTop = 0
    }
  }, [events, userScrolled])

  function handleScroll() {
    const el = containerRef.current
    if (!el) return
    // If user scrolled away from top, stop auto-scrolling
    setUserScrolled(el.scrollTop > 10)
  }

  return (
    <div
      className="glass-card overflow-hidden"
      style={fullPage ? {} : {}}
    >
      {/* Header */}
      <div
        className="flex items-center justify-between px-5 py-3"
        style={{ borderBottom: '1px solid rgba(148,163,184,0.07)' }}
      >
        <div className="flex items-center gap-3">
          <Activity className="w-4 h-4 text-brand-400" />
          <div>
            <span className="text-sm font-semibold text-slate-200 font-display">Live Event Stream</span>
            <span className="text-[11px] text-slate-600 ml-2">last {MAX_EVENTS} events</span>
          </div>
          {/* Connection indicator */}
          <div className="flex items-center gap-1.5">
            {connected ? (
              <>
                <Wifi className="w-3.5 h-3.5 text-brand-400" />
                <span className="text-[10px] text-brand-400 font-medium">LIVE</span>
                <span className="w-1.5 h-1.5 rounded-full bg-brand-400 animate-pulse" />
              </>
            ) : (
              <>
                <WifiOff className="w-3.5 h-3.5 text-slate-500" />
                <span className="text-[10px] text-slate-500">Reconnecting…</span>
              </>
            )}
          </div>
        </div>

        {!fullPage && (
          <button
            onClick={() => setCollapsed(c => !c)}
            className="p-1 text-slate-500 hover:text-slate-300 transition-colors"
            title={collapsed ? 'Expand' : 'Collapse'}
          >
            {collapsed ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
          </button>
        )}
      </div>

      {/* Column headers */}
      {!collapsed && (
        <>
          <div
            className="flex items-center gap-3 px-4 py-2"
            style={{ background: 'rgba(255,255,255,0.01)', borderBottom: '1px solid rgba(148,163,184,0.05)' }}
          >
            <span className="text-[10px] text-slate-600 uppercase tracking-wider w-16 shrink-0">Time</span>
            <span className="text-[10px] text-slate-600 uppercase tracking-wider shrink-0 w-24">Kind</span>
            <span className="text-[10px] text-slate-600 uppercase tracking-wider w-[100px] shrink-0">Agent</span>
            <span className="text-[10px] text-slate-600 uppercase tracking-wider flex-1">Tool / Model</span>
            <span className="text-[10px] text-slate-600 uppercase tracking-wider w-20 shrink-0">Risk</span>
            <span className="text-[10px] text-slate-600 uppercase tracking-wider w-14 text-right shrink-0">Latency</span>
          </div>

          {/* Event list */}
          <div
            ref={containerRef}
            onScroll={handleScroll}
            style={{ maxHeight: fullPage ? 'calc(100vh - 200px)' : `${maxHeight}px`, overflowY: 'auto' }}
          >
            {events.length === 0 ? (
              <div className="flex flex-col items-center justify-center py-12 text-center">
                <Activity className="w-8 h-8 text-slate-600 mb-3" />
                <div className="text-sm text-slate-500">
                  {connected ? 'Waiting for events…' : 'Connecting to event stream…'}
                </div>
                <div className="text-xs text-slate-600 mt-1">
                  Events will appear here in real-time as agents make API calls.
                </div>
              </div>
            ) : (
              events.map(event => (
                <EventStreamRow key={event.id} event={event} />
              ))
            )}
          </div>

          {/* Footer */}
          {events.length > 0 && (
            <div
              className="flex items-center justify-between px-4 py-2 text-[10px] text-slate-600"
              style={{ borderTop: '1px solid rgba(148,163,184,0.05)' }}
            >
              <span>{events.length} events buffered</span>
              {userScrolled && (
                <button
                  onClick={() => {
                    setUserScrolled(false)
                    if (containerRef.current) containerRef.current.scrollTop = 0
                  }}
                  className={clsx(
                    'text-brand-400 hover:text-brand-300 transition-colors',
                  )}
                >
                  Jump to latest
                </button>
              )}
            </div>
          )}
        </>
      )}
    </div>
  )
}
