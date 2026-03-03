import { NavLink } from 'react-router-dom'
import { LayoutDashboard, Zap, Users, DollarSign, FileText } from 'lucide-react'
import { useAuth } from '../../context/AuthContext'

const NAV = [
  { to: '/overview', icon: LayoutDashboard, label: 'Overview' },
  { to: '/events', icon: Zap, label: 'Events' },
  { to: '/agents', icon: Users, label: 'Agents' },
  { to: '/costs', icon: DollarSign, label: 'Costs' },
  { to: '/reports', icon: FileText, label: 'Reports' },
]

export default function Sidebar() {
  const { darkMode, toggleDarkMode } = useAuth()

  return (
    <aside className="w-56 shrink-0 flex flex-col border-r border-slate-200 dark:border-[#272737] bg-white dark:bg-[#11111b] h-screen sticky top-0">
      {/* Brand */}
      <div className="px-5 py-5 border-b border-slate-200 dark:border-[#272737]">
        <div className="flex items-center gap-2">
          <Zap className="w-5 h-5 text-indigo-500" />
          <span className="font-bold text-slate-900 dark:text-slate-100 text-sm tracking-tight">govrix scout</span>
        </div>
        <span className="text-[10px] text-slate-400 font-medium uppercase tracking-widest mt-0.5 block">Open Source</span>
      </div>

      {/* Nav */}
      <nav className="flex-1 px-3 py-4 space-y-0.5">
        {NAV.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              `flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-sm font-medium transition-colors ${
                isActive
                  ? 'bg-indigo-50 dark:bg-indigo-950/50 text-indigo-600 dark:text-indigo-400'
                  : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-[#1a1a2e]'
              }`
            }
          >
            <Icon className="w-4 h-4" />
            {label}
          </NavLink>
        ))}
      </nav>

      {/* Footer */}
      <div className="px-4 py-4 border-t border-slate-200 dark:border-[#272737]">
        <button
          onClick={toggleDarkMode}
          className="w-full text-left text-xs text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 transition-colors"
        >
          {darkMode ? '☀ Light mode' : '☾ Dark mode'}
        </button>
      </div>
    </aside>
  )
}
