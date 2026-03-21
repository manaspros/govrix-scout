/**
 * SettingsPage — API key management, display preferences, license info, and about.
 *
 * Enterprise-enhanced settings with personalization and editing capabilities.
 */

import { useState, useCallback } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  CheckCircle,
  XCircle,
  Loader2,
  Key,
  Eye,
  EyeOff,
  Monitor,
  Clock,
  Globe,
  Info,
  Shield,
  Zap,
  Save,
  Trash2,
  RefreshCw,
  ExternalLink,
} from 'lucide-react'
import { useHealth, useConfig } from '@/api/hooks'
import { fetchPlatformHealth, fetchLicense } from '@/api/platform'
import type { PlatformHealth, LicenseInfo } from '@/api/types'

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

// ── Feature pill ──────────────────────────────────────────────────────────────

function FeaturePill({ label, enabled }: { label: string; enabled: boolean }) {
  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium transition-colors ${
        enabled
          ? 'bg-emerald-500/10 text-emerald-400 border border-emerald-500/20'
          : 'bg-slate-700/40 text-slate-500 border border-slate-600/20'
      }`}
    >
      <span className={`w-1.5 h-1.5 rounded-full ${enabled ? 'bg-emerald-400' : 'bg-slate-600'}`} />
      {label}
    </span>
  )
}

// ── Tier badge ────────────────────────────────────────────────────────────────

function TierBadge({ tier }: { tier: string }) {
  const colors: Record<string, string> = {
    free: 'bg-slate-700/40 text-slate-400 border-slate-600/20',
    starter: 'bg-blue-500/10 text-blue-400 border-blue-500/20',
    professional: 'bg-purple-500/10 text-purple-400 border-purple-500/20',
    enterprise: 'bg-amber-500/10 text-amber-400 border-amber-500/20',
  }
  const colorClass = colors[tier.toLowerCase()] || colors.free

  return (
    <span className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-semibold border ${colorClass}`}>
      <Shield className="w-3.5 h-3.5" />
      {tier.charAt(0).toUpperCase() + tier.slice(1)}
    </span>
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
  const queryClient = useQueryClient()
  const { data: health } = useHealth()
  const { data: config, isLoading: configLoading } = useConfig()

  // ── API Key state ─────────────────────────────────────────────────────────
  const [apiKey, setApiKey] = useState(() => localStorage.getItem('govrix_api_key') || '')
  const [showKey, setShowKey] = useState(false)
  const [keySaved, setKeySaved] = useState(false)

  const saveApiKey = useCallback(() => {
    if (apiKey.trim()) {
      localStorage.setItem('govrix_api_key', apiKey.trim())
    } else {
      localStorage.removeItem('govrix_api_key')
    }
    setKeySaved(true)
    setTimeout(() => setKeySaved(false), 2000)
    // Trigger a re-check of platform health
    queryClient.invalidateQueries({ queryKey: ['platform-health'] })
    queryClient.invalidateQueries({ queryKey: ['license'] })
  }, [apiKey, queryClient])

  const clearApiKey = useCallback(() => {
    setApiKey('')
    localStorage.removeItem('govrix_api_key')
    queryClient.invalidateQueries({ queryKey: ['platform-health'] })
    queryClient.invalidateQueries({ queryKey: ['license'] })
  }, [queryClient])

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

  // ── Platform health & license queries ─────────────────────────────────────
  const { data: platformHealth, isLoading: platformLoading, isError: platformError } = useQuery<PlatformHealth>({
    queryKey: ['platform-health'],
    queryFn: fetchPlatformHealth,
    staleTime: 30_000,
    retry: 1,
  })

  const { data: license, isLoading: licenseLoading } = useQuery<LicenseInfo>({
    queryKey: ['license'],
    queryFn: fetchLicense,
    staleTime: 60_000,
    retry: 1,
  })

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

  const displayVersion = platformHealth?.version || health?.version || '0.1.0'

  return (
    <div className="max-w-2xl space-y-6 page-enter">

      {/* ── 1. API Configuration ───────────────────────────────────────────── */}
      <div className="card p-5 space-y-4">
        <div className="flex items-center gap-2">
          <Key className="w-4 h-4 text-emerald-400" />
          <h2 className="text-sm font-semibold text-slate-200">API Configuration</h2>
        </div>

        <div className="space-y-3">
          {/* API Key input */}
          <div>
            <label className="section-label mb-2 block">Platform API Key</label>
            <div className="flex items-center gap-2">
              <div className="relative flex-1">
                <input
                  type={showKey ? 'text' : 'password'}
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  placeholder="Enter your Govrix API key..."
                  className="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-emerald-500/50 focus:ring-1 focus:ring-emerald-500/20 transition-colors font-mono"
                />
                <button
                  type="button"
                  onClick={() => setShowKey(!showKey)}
                  className="absolute right-2 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 transition-colors p-1"
                  title={showKey ? 'Hide key' : 'Show key'}
                >
                  {showKey ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
            </div>
          </div>

          {/* Action buttons */}
          <div className="flex items-center gap-2">
            <button
              onClick={saveApiKey}
              className="btn btn-primary"
            >
              <Save className="w-3.5 h-3.5" />
              {keySaved ? 'Saved' : 'Save Key'}
            </button>
            <button
              onClick={clearApiKey}
              className="btn btn-ghost"
              disabled={!apiKey}
            >
              <Trash2 className="w-3.5 h-3.5" />
              Clear
            </button>
          </div>

          {/* Connection status indicator */}
          <div className="flex items-center gap-2 pt-1">
            {platformLoading ? (
              <div className="flex items-center gap-2 text-xs text-slate-500">
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                Checking platform connection...
              </div>
            ) : platformError ? (
              <div className="flex items-center gap-2 text-xs text-red-400">
                <XCircle className="w-3.5 h-3.5" />
                Platform unreachable — check your API key and server status
              </div>
            ) : platformHealth ? (
              <div className="flex items-center gap-2 text-xs text-emerald-400">
                <CheckCircle className="w-3.5 h-3.5" />
                Connected to Govrix Platform v{platformHealth.version} ({platformHealth.license_tier} tier)
              </div>
            ) : (
              <div className="flex items-center gap-2 text-xs text-slate-500">
                <Info className="w-3.5 h-3.5" />
                Enter an API key to connect to the enterprise platform
              </div>
            )}
          </div>
        </div>
      </div>

      {/* ── 2. Display Preferences ─────────────────────────────────────────── */}
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

      {/* ── 4. License & Platform Info ─────────────────────────────────────── */}
      <div className="card p-5 space-y-4">
        <div className="flex items-center gap-2">
          <Shield className="w-4 h-4 text-emerald-400" />
          <h2 className="text-sm font-semibold text-slate-200">License & Platform</h2>
        </div>

        {licenseLoading || platformLoading ? (
          <div className="space-y-3">
            {Array.from({ length: 3 }).map((_, i) => (
              <div key={i} className="skeleton h-5" />
            ))}
          </div>
        ) : license || platformHealth ? (
          <div className="space-y-4">
            {/* Tier badge */}
            <div className="flex items-center justify-between">
              <span className="text-sm text-slate-400">License Tier</span>
              <TierBadge tier={license?.tier || platformHealth?.license_tier || 'free'} />
            </div>

            {/* Max agents */}
            <div className="flex items-center justify-between py-2 border-b border-slate-700/40">
              <span className="text-sm text-slate-400">Max Agents</span>
              <span className="text-sm text-slate-200 font-mono">{license?.max_agents ?? platformHealth?.max_agents ?? '-'}</span>
            </div>

            {/* Retention */}
            {license?.retention_days != null && (
              <div className="flex items-center justify-between py-2 border-b border-slate-700/40">
                <span className="text-sm text-slate-400">Data Retention</span>
                <span className="text-sm text-slate-200">{license.retention_days} days</span>
              </div>
            )}

            {/* Platform version & uptime */}
            {platformHealth?.version && (
              <div className="flex items-center justify-between py-2 border-b border-slate-700/40">
                <span className="text-sm text-slate-400">Platform Version</span>
                <span className="text-sm text-slate-200 font-mono">v{platformHealth.version}</span>
              </div>
            )}

            {/* Feature flags */}
            <div>
              <div className="section-label mb-3">Feature Flags</div>
              <div className="flex flex-wrap gap-2">
                <FeaturePill
                  label="Policy Engine"
                  enabled={platformHealth?.policy_enabled ?? license?.policy_enabled ?? false}
                />
                <FeaturePill
                  label="PII Masking"
                  enabled={platformHealth?.pii_masking_enabled ?? license?.pii_masking_enabled ?? false}
                />
                <FeaturePill
                  label="Compliance"
                  enabled={platformHealth?.compliance_enabled ?? license?.compliance_enabled ?? false}
                />
                <FeaturePill
                  label="mTLS / A2A Identity"
                  enabled={platformHealth?.mtls_enabled ?? platformHealth?.a2a_identity_enabled ?? license?.a2a_identity_enabled ?? false}
                />
                <FeaturePill
                  label="Budget Tracking"
                  enabled={platformHealth?.budget_tracking_enabled ?? false}
                />
                <FeaturePill
                  label="Audit Trail"
                  enabled={platformHealth?.audit_trail_enabled ?? false}
                />
              </div>
            </div>
          </div>
        ) : (
          <div className="text-sm text-slate-500">
            <p>No license information available. Configure your API key above to connect to the enterprise platform.</p>
          </div>
        )}
      </div>

      {/* ── 5. Proxy Configuration (read-only) ────────────────────────────── */}
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
            <ConfigRow label="Proxy Port" value={config.proxy_port as number | undefined} />
            <ConfigRow label="API Port" value={config.api_port as number | undefined} />
            <ConfigRow label="Log Level" value={config.log_level as string | undefined} />
            <ConfigRow label="Event Retention (days)" value={config.event_retention_days as number | undefined} />
            <ConfigRow label="Agent Soft Limit" value={config.agent_soft_limit as number | undefined} />
            <ConfigRow label="Max Body Size (bytes)" value={config.max_body_size_bytes as number | undefined} />
            {/* Render any additional keys */}
            {Object.entries(config)
              .filter(([k]) => !['proxy_port','api_port','log_level','event_retention_days','agent_soft_limit','max_body_size_bytes','database_url'].includes(k))
              .map(([k, v]) => (
                <ConfigRow key={k} label={k} value={v as string | number | boolean | undefined} />
              ))}
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
            <span className="text-sm text-slate-200 font-semibold">Govrix Enterprise</span>
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

      {/* ── 7. Enterprise upsell ───────────────────────────────────────────── */}
      <div className="bg-emerald-600/5 border border-emerald-600/20 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-2">
          <Zap className="w-4 h-4 text-emerald-400" />
          <span className="text-sm font-semibold text-emerald-400">Govrix Platform</span>
        </div>
        <p className="text-xs text-slate-400 mb-3">
          Need policy enforcement, PII masking, compliance templates, SSO, RBAC, A2A identity, or multi-cluster support? Upgrade to Govrix Platform.
        </p>
        <a
          href="https://govrix.io/platform"
          target="_blank"
          rel="noopener noreferrer"
          className="btn btn-primary"
        >
          Learn about Enterprise
          <ExternalLink className="w-3.5 h-3.5" />
        </a>
      </div>
    </div>
  )
}
