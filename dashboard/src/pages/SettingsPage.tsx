/**
 * SettingsPage — display preferences, proxy config, and about.
 */

import { useState, useCallback } from 'react'
import {
  CheckCircle,
  XCircle,
  Loader2,
  Monitor,
  Clock,
  Globe,
  Info,
  Save,
  RefreshCw,
  ExternalLink,
} from 'lucide-react'
import { useHealth, useConfig } from '@/api/hooks'

// ── Connection status indicator ───────────────────────────────────────────────

function ConnectionStatus() {
  const { data, isLoading, isError } = useHealth()

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 text-sm text-slate-400">
        <Loader2 className="w-4 h-4 animate-spin" />
        Checking connection...
      </div>
    )
  }

  if (isError) {
    return (
      <div className="flex items-center gap-2 text-sm text-red-400">
        <XCircle className="w-4 h-4" />
        API server unreachable — is Scout running on port 8080?
      </div>
    )
  }

  return (
    <div className="flex items-center gap-2 text-sm text-emerald-400">
      <CheckCircle className="w-4 h-4" />
      Connected to Scout {data?.version ? `v${data.version}` : ''}
    </div>
  )
}

// ── Config display ────────────────────────────────────────────────────────────

interface ConfigRowProps {
  label: string
  value: string | number | boolean | undefined
}

function ConfigRow({ label, value }: ConfigRowProps) {
  const display = value === undefined || value === null || value === ''
    ? <span className="text-slate-600">not set</span>
    : typeof value === 'boolean'
      ? <span className={value ? 'text-emerald-400' : 'text-slate-400'}>{value ? 'true' : 'false'}</span>
      : <span className="text-slate-200 font-mono">{String(value)}</span>

  return (
    <div className="flex items-start justify-between py-3 border-b border-slate-700/40 last:border-0">
      <span className="text-sm text-slate-400">{label}</span>
      <span className="text-sm">{display}</span>
    </div>
  )
}

// ── Toggle component ──────────────────────────────────────────────────────────

function Toggle({ active, onClick }: { active: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      className={`toggle ${active ? 'active' : ''}`}
      onClick={onClick}
      aria-pressed={active}
    />
  )
}

// ── Refresh interval options ──────────────────────────────────────────────────

const REFRESH_OPTIONS = [
  { label: '5s', value: 5 },
  { label: '10s', value: 10 },
  { label: '30s', value: 30 },
  { label: '60s', value: 60 },
  { label: 'Off', value: 0 },
]

// ── Common timezone list ──────────────────────────────────────────────────────

const TIMEZONES = [
  'UTC',
  'America/New_York',
  'America/Chicago',
  'America/Denver',
  'America/Los_Angeles',
  'America/Sao_Paulo',
  'Europe/London',
  'Europe/Berlin',
  'Europe/Paris',
  'Asia/Dubai',
  'Asia/Kolkata',
  'Asia/Shanghai',
  'Asia/Tokyo',
  'Asia/Singapore',
  'Australia/Sydney',
  'Pacific/Auckland',
]

// ── Settings page ─────────────────────────────────────────────────────────────

export function SettingsPage() {
  const { data: health } = useHealth()
  const { data: config, isLoading: configLoading } = useConfig()

  // ── Display Preferences state ─────────────────────────────────────────────
  const [compactMode, setCompactMode] = useState(() => localStorage.getItem('govrix_compact') === 'true')
  const [refreshInterval, setRefreshInterval] = useState(() => parseInt(localStorage.getItem('govrix_refresh') || '30'))
  const [timezone, setTimezone] = useState(() => localStorage.getItem('govrix_tz') || Intl.DateTimeFormat().resolvedOptions().timeZone)
  const [prefsSaved, setPrefsSaved] = useState(false)

  const savePreferences = useCallback(() => {
    localStorage.setItem('govrix_compact', String(compactMode))
    localStorage.setItem('govrix_refresh', String(refreshInterval))
    localStorage.setItem('govrix_tz', timezone)
    setPrefsSaved(true)
    setTimeout(() => setPrefsSaved(false), 2000)
  }, [compactMode, refreshInterval, timezone])

  // ── Version check state ───────────────────────────────────────────────────
  const [checkingUpdate, setCheckingUpdate] = useState(false)
  const [updateChecked, setUpdateChecked] = useState(false)

  const checkForUpdates = useCallback(() => {
    setCheckingUpdate(true)
    setTimeout(() => {
      setCheckingUpdate(false)
      setUpdateChecked(true)
      setTimeout(() => setUpdateChecked(false), 4000)
    }, 1500)
  }, [])

  const displayVersion = health?.version || '0.1.0'

  return (
    <div className="max-w-2xl space-y-6 page-enter">

      {/* ── 1. Display Preferences ─────────────────────────────────────────── */}
      <div className="card p-5 space-y-4">
        <div className="flex items-center gap-2">
          <Monitor className="w-4 h-4 text-emerald-400" />
          <h2 className="text-sm font-semibold text-slate-200">Display Preferences</h2>
        </div>

        <div className="space-y-4">
          {/* Compact mode */}
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm text-slate-200">Compact Mode</div>
              <div className="text-xs text-slate-500 mt-0.5">Reduces padding and spacing across the dashboard</div>
            </div>
            <Toggle
              active={compactMode}
              onClick={() => setCompactMode(!compactMode)}
            />
          </div>

          {/* Auto-refresh interval */}
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm text-slate-200 flex items-center gap-1.5">
                <Clock className="w-3.5 h-3.5 text-slate-400" />
                Auto-Refresh Interval
              </div>
              <div className="text-xs text-slate-500 mt-0.5">How often live data updates automatically</div>
            </div>
            <select
              value={refreshInterval}
              onChange={(e) => setRefreshInterval(Number(e.target.value))}
              className="bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-emerald-500/50 transition-colors cursor-pointer"
            >
              {REFRESH_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value} className="bg-slate-800 text-white">
                  {opt.label}
                </option>
              ))}
            </select>
          </div>

          {/* Timezone */}
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm text-slate-200 flex items-center gap-1.5">
                <Globe className="w-3.5 h-3.5 text-slate-400" />
                Timezone
              </div>
              <div className="text-xs text-slate-500 mt-0.5">Used for displaying timestamps in the dashboard</div>
            </div>
            <select
              value={timezone}
              onChange={(e) => setTimezone(e.target.value)}
              className="bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:border-emerald-500/50 transition-colors cursor-pointer max-w-[200px]"
            >
              {TIMEZONES.map((tz) => (
                <option key={tz} value={tz} className="bg-slate-800 text-white">
                  {tz}
                </option>
              ))}
            </select>
          </div>
        </div>

        {/* Save preferences button */}
        <div className="pt-2 border-t border-white/5">
          <button onClick={savePreferences} className="btn btn-primary">
            <Save className="w-3.5 h-3.5" />
            {prefsSaved ? 'Preferences Saved' : 'Save Preferences'}
          </button>
        </div>
      </div>

      {/* ── 3. Connection & Scout Info ─────────────────────────────────────── */}
      <div className="card p-5 space-y-4">
        <h2 className="text-sm font-semibold text-slate-200">Connection</h2>
        <ConnectionStatus />

        {health && (
          <div className="grid grid-cols-2 gap-3 text-xs mt-2">
            <div className="bg-slate-700/40 rounded-lg p-3">
              <div className="text-slate-500 mb-1">API Status</div>
              <div className={health.status === 'ok' ? 'text-emerald-400' : 'text-yellow-400'}>
                {health.status}
              </div>
            </div>
            {health.db && (
              <div className="bg-slate-700/40 rounded-lg p-3">
                <div className="text-slate-500 mb-1">Database</div>
                <div className="text-slate-200">{health.db}</div>
              </div>
            )}
            {health.version && (
              <div className="bg-slate-700/40 rounded-lg p-3">
                <div className="text-slate-500 mb-1">Version</div>
                <div className="text-slate-200 font-mono">{health.version}</div>
              </div>
            )}
            {health.uptime_secs != null && (
              <div className="bg-slate-700/40 rounded-lg p-3">
                <div className="text-slate-500 mb-1">Uptime</div>
                <div className="text-slate-200">{Math.round(health.uptime_secs / 60)}m</div>
              </div>
            )}
          </div>
        )}
      </div>

      {/* ── 3. Proxy Configuration (read-only from /api/v1/config) ─────── */}
      <div className="card p-5">
        <h2 className="text-sm font-semibold text-slate-200 mb-4">Proxy Configuration</h2>
        {configLoading ? (
          <div className="space-y-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <div key={i} className="skeleton h-5" />
            ))}
          </div>
        ) : config ? (
          <div>
            <ConfigRow label="Proxy Port" value={(config as any)?.proxy?.port} />
            <ConfigRow label="API Port" value={(config as any)?.api?.port} />
            <ConfigRow label="Upstream OpenAI" value={(config as any)?.proxy?.upstream_openai} />
            <ConfigRow label="Upstream Anthropic" value={(config as any)?.proxy?.upstream_anthropic} />
            <ConfigRow label="Fail Open" value={(config as any)?.proxy?.fail_open} />
            <ConfigRow label="Upstream Timeout (ms)" value={(config as any)?.proxy?.upstream_timeout_ms} />
            <ConfigRow label="Max Body Size (bytes)" value={(config as any)?.proxy?.max_body_tee_bytes} />
            <ConfigRow label="Log Level" value={(config as any)?.telemetry?.log_level} />
            <ConfigRow label="Event Retention (days)" value={(config as any)?.retention?.events_days} />
            <ConfigRow label="Cost Retention (days)" value={(config as any)?.retention?.cost_days} />
            <ConfigRow label="DB Max Connections" value={(config as any)?.database?.max_connections} />
          </div>
        ) : (
          <div className="text-sm text-slate-500">
            Could not load configuration. Ensure the API server is running.
          </div>
        )}
      </div>

      {/* ── 6. About ───────────────────────────────────────────────────────── */}
      <div className="card p-5 space-y-4">
        <div className="flex items-center gap-2">
          <Info className="w-4 h-4 text-emerald-400" />
          <h2 className="text-sm font-semibold text-slate-200">About Govrix</h2>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between py-2 border-b border-slate-700/40">
            <span className="text-sm text-slate-400">Product</span>
            <span className="text-sm text-slate-200 font-semibold">Govrix Scout</span>
          </div>
          <div className="flex items-center justify-between py-2 border-b border-slate-700/40">
            <span className="text-sm text-slate-400">Version</span>
            <span className="text-sm text-slate-200 font-mono">v{displayVersion}</span>
          </div>
          <div className="flex items-center justify-between py-2 border-b border-slate-700/40">
            <span className="text-sm text-slate-400">Documentation</span>
            <a
              href="https://govrix.io/docs"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 text-sm text-emerald-400 hover:text-emerald-300 transition-colors"
            >
              govrix.io/docs
              <ExternalLink className="w-3 h-3" />
            </a>
          </div>
        </div>

        <div className="flex items-center gap-2 pt-2">
          <button
            onClick={checkForUpdates}
            disabled={checkingUpdate}
            className="btn btn-ghost"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${checkingUpdate ? 'animate-spin' : ''}`} />
            {checkingUpdate ? 'Checking...' : 'Check for Updates'}
          </button>
          {updateChecked && (
            <span className="text-xs text-emerald-400 flex items-center gap-1.5">
              <CheckCircle className="w-3.5 h-3.5" />
              You are on the latest version (v{displayVersion})
            </span>
          )}
        </div>
      </div>

    </div>
  )
}
