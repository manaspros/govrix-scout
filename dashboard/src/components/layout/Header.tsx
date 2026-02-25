import { useLocation } from 'react-router-dom'
import { CheckCircle, AlertTriangle, XCircle, Loader2 } from 'lucide-react'
import { useHealth } from '@/api/hooks'

const PAGE_TITLES: Record<string, string> = {
  '/overview': 'Overview',
  '/agents':   'Agents',
  '/events':   'Events',
  '/costs':    'Costs',
  '/reports':  'Reports',
  '/settings': 'Settings',
}

function HealthIndicator() {
  const { data, isLoading, isError } = useHealth()

  if (isLoading) {
    return (
      <div className="flex items-center gap-1.5 text-xs text-slate-400">
        <Loader2 className="w-3.5 h-3.5 animate-spin" />
        <span>Connecting…</span>
      </div>
    )
  }

  if (isError) {
    return (
      <div className="flex items-center gap-1.5 text-xs text-red-400">
        <XCircle className="w-3.5 h-3.5" />
        <span>API Offline</span>
      </div>
    )
  }

  if (data?.status === 'ok') {
    return (
      <div className="flex items-center gap-1.5 text-xs text-emerald-400">
        <CheckCircle className="w-3.5 h-3.5" />
        <span>Connected</span>
        {data.version && <span className="text-slate-500">v{data.version}</span>}
      </div>
    )
  }

  return (
    <div className="flex items-center gap-1.5 text-xs text-yellow-400">
      <AlertTriangle className="w-3.5 h-3.5" />
      <span>Degraded</span>
    </div>
  )
}

export function Header() {
  const location = useLocation()
  const match = Object.keys(PAGE_TITLES).find(path => location.pathname.startsWith(path))
  const title = match ? PAGE_TITLES[match] : 'Scout'

  return (
    <header className="h-14 flex items-center justify-between px-6 border-b border-slate-700/60 bg-slate-900/80 backdrop-blur-sm shrink-0">
      <h1 className="text-base font-semibold text-slate-100">{title}</h1>
      <HealthIndicator />
    </header>
  )
}
