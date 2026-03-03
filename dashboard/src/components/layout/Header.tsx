import { useLocation } from 'react-router-dom'
import { useHealth } from '../../api/hooks'

const PAGE_META: Record<string, { title: string; description: string }> = {
  '/overview': { title: 'Overview', description: 'System health and activity at a glance' },
  '/events': { title: 'Events', description: 'Real-time agent event stream' },
  '/agents': { title: 'Agents', description: 'Registered agents and their activity' },
  '/costs': { title: 'Cost Analytics', description: 'Token usage and spend breakdown' },
  '/reports': { title: 'Reports', description: 'Generate and download audit reports' },
  '/governance': { title: 'Governance', description: 'Risk posture and policy enforcement' },
  '/sessions': { title: 'Session Replay', description: 'Real-time session recording and forensics' },
  '/compliance': { title: 'Compliance', description: 'Framework compliance and audit evidence' },
  '/settings': { title: 'Settings', description: 'System configuration and integrations' },
}

export default function Header() {
  const { pathname } = useLocation()
  const meta = PAGE_META[pathname] ?? { title: 'Govrix Scout', description: '' }
  const { data: health } = useHealth()

  const statusColor =
    health?.status === 'ok' ? 'bg-emerald-500' :
    health?.status === 'degraded' ? 'bg-amber-500' :
    'bg-red-500'

  return (
    <header className="h-14 px-6 flex items-center justify-between border-b border-slate-200 dark:border-[#272737] bg-white dark:bg-[#11111b] sticky top-0 z-10">
      <div>
        <h1 className="text-base font-semibold text-slate-900 dark:text-slate-100">{meta.title}</h1>
        <p className="text-xs text-slate-500">{meta.description}</p>
      </div>
      <div className="flex items-center gap-2">
        <span className={`w-2 h-2 rounded-full ${statusColor}`} />
        <span className="text-xs text-slate-500">
          {health ? `v${health.version}` : 'Connecting...'}
        </span>
      </div>
    </header>
  )
}
