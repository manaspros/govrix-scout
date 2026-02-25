import { NavLink } from 'react-router-dom'
import {
  LayoutDashboard,
  Bot,
  Activity,
  DollarSign,
  FileText,
  Settings,
  ExternalLink,
  Zap,
} from 'lucide-react'
import { clsx } from 'clsx'

const NAV_ITEMS = [
  { to: '/overview', label: 'Overview',  icon: LayoutDashboard },
  { to: '/agents',   label: 'Agents',    icon: Bot },
  { to: '/events',   label: 'Events',    icon: Activity },
  { to: '/costs',    label: 'Costs',     icon: DollarSign },
  { to: '/reports',  label: 'Reports',   icon: FileText },
  { to: '/settings', label: 'Settings',  icon: Settings },
]

export function Sidebar() {
  return (
    <aside className="w-60 min-h-screen bg-slate-900 border-r border-slate-700/60 flex flex-col shrink-0">
      {/* Logo */}
      <div className="px-5 py-5 border-b border-slate-700/60">
        <div className="flex items-center gap-2.5">
          <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-brand-600">
            <Zap className="w-4 h-4 text-white" />
          </div>
          <div>
            <div className="text-sm font-bold text-white tracking-tight">Scout</div>
            <div className="text-[10px] text-slate-400 -mt-0.5">by Govrix</div>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 py-4 space-y-0.5">
        {NAV_ITEMS.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              clsx(
                'flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors',
                isActive
                  ? 'bg-brand-600/20 text-brand-400 ring-1 ring-brand-600/30'
                  : 'text-slate-400 hover:bg-slate-800 hover:text-slate-200',
              )
            }
          >
            {({ isActive }) => (
              <>
                <Icon className={clsx('w-4 h-4 shrink-0', isActive ? 'text-brand-400' : 'text-slate-500')} />
                {label}
              </>
            )}
          </NavLink>
        ))}
      </nav>

      {/* Footer */}
      <div className="px-3 py-4 border-t border-slate-700/60 space-y-2">
        <a
          href="https://govrix.io/platform"
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-semibold text-brand-400 bg-brand-600/10 ring-1 ring-brand-600/30 hover:bg-brand-600/20 transition-colors"
        >
          <Zap className="w-3.5 h-3.5" />
          Upgrade to Platform
          <ExternalLink className="w-3 h-3 ml-auto" />
        </a>
        <div className="px-3 text-[10px] text-slate-600">
          Govrix Scout OSS
        </div>
      </div>
    </aside>
  )
}
