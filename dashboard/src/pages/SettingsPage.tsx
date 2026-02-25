/**
 * SettingsPage — Shows current proxy config, connection status, and version info.
 *
 * NOTE: Authentication / access control is coming in Govrix Platform.
 * The OSS version is designed for local / trusted network use only.
 */

import { CheckCircle, XCircle, Loader2, Lock, Zap } from 'lucide-react'
import { useHealth, useConfig } from '@/api/hooks'

// ── Connection status indicator ───────────────────────────────────────────────

function ConnectionStatus() {
  const { data, isLoading, isError } = useHealth()

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 text-sm text-slate-400">
        <Loader2 className="w-4 h-4 animate-spin" />
        Checking connection…
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

// ── Settings page ─────────────────────────────────────────────────────────────

export function SettingsPage() {
  const { data: health } = useHealth()
  const { data: config, isLoading: configLoading } = useConfig()

  return (
    <div className="max-w-2xl space-y-6">
      {/* Auth notice */}
      <div className="flex items-start gap-3 p-4 bg-slate-800 border border-slate-700/60 rounded-xl">
        <Lock className="w-4 h-4 text-slate-400 mt-0.5 shrink-0" />
        <div className="text-xs text-slate-400">
          <span className="font-semibold text-slate-200">Authentication is not enabled in Scout OSS.</span>
          {' '}This dashboard is intended for local or trusted network use. SSO, RBAC, and audit logging are available in{' '}
          <a
            href="https://govrix.io/platform"
            target="_blank"
            rel="noopener noreferrer"
            className="text-brand-400 hover:text-brand-300 underline"
          >
            Govrix Platform
          </a>.
        </div>
      </div>

      {/* Connection & version */}
      <div className="bg-slate-800 rounded-xl border border-slate-700/60 p-5 space-y-4">
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

      {/* Config */}
      <div className="bg-slate-800 rounded-xl border border-slate-700/60 p-5">
        <h2 className="text-sm font-semibold text-slate-200 mb-4">Configuration</h2>
        {configLoading ? (
          <div className="animate-pulse space-y-3">
            {Array.from({ length: 6 }).map((_, i) => (
              <div key={i} className="h-5 bg-slate-700/50 rounded" />
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

      {/* Enterprise upsell */}
      <div className="bg-brand-600/10 border border-brand-600/30 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-2">
          <Zap className="w-4 h-4 text-brand-400" />
          <span className="text-sm font-semibold text-brand-400">Govrix Platform</span>
        </div>
        <p className="text-xs text-slate-400 mb-3">
          Need policy enforcement, PII masking, compliance templates, SSO, RBAC, A2A identity, or multi-cluster support? Upgrade to Govrix Platform.
        </p>
        <a
          href="https://govrix.io/platform"
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 px-4 py-2 text-sm font-semibold bg-brand-600 text-white rounded-lg hover:bg-brand-500 transition-colors"
        >
          Learn about Enterprise →
        </a>
      </div>
    </div>
  )
}
