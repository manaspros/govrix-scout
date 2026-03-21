import { NavLink, useLocation } from 'react-router-dom'
import {
  LayoutDashboard,
  Bot,
  Activity,
  DollarSign,
  FileText,
  Settings,
  ShieldAlert,
  ScrollText,
  ShieldCheck,
  Clapperboard,
  Wallet,
  Power,
  ScanEye,
  Shield,
  Globe,
  GitBranch,
  Radio,
} from 'lucide-react'
import { clsx } from 'clsx'

interface NavItem {
  to: string
  label: string
  icon: React.ComponentType<{ className?: string }>
}

const OBSERVABILITY_ITEMS: NavItem[] = [
  { to: '/overview',  label: 'Overview',     icon: LayoutDashboard },
  { to: '/agents',    label: 'Agents',       icon: Bot },
  { to: '/events',    label: 'Events',       icon: Activity },
  { to: '/traces',    label: 'Traces',       icon: GitBranch },
  { to: '/stream',    label: 'Live Stream',  icon: Radio },
  { to: '/costs',     label: 'Costs',        icon: DollarSign },
  { to: '/reports',   label: 'Reports',      icon: FileText },
]

const GOVERNANCE_ITEMS: NavItem[] = [
  { to: '/risk',        label: 'Risk Overview', icon: ShieldAlert },
  { to: '/policies',    label: 'Policies',      icon: ScrollText },
  { to: '/compliance',  label: 'Compliance',    icon: ShieldCheck },
  { to: '/sessions',    label: 'Sessions',      icon: Clapperboard },
  { to: '/budget',      label: 'Budget',        icon: Wallet },
  { to: '/kill-switch', label: 'Kill Switch',   icon: Power },
  { to: '/pii',         label: 'PII Activity',  icon: ScanEye },
  { to: '/eu-ai-act',   label: 'EU AI Act',     icon: Globe },
]

function NavSection({ label, items }: { label: string; items: NavItem[] }) {
  return (
    <div className="mb-1">
      <div className="px-4 pt-5 pb-2 flex items-center gap-2">
        <span className="section-label">{label}</span>
        <div
          className="flex-1 h-px"
          style={{ background: 'linear-gradient(90deg, rgba(71,85,105,0.28) 0%, transparent 100%)' }}
        />
      </div>
      <div className="space-y-px px-2">
        {items.map(({ to, label: itemLabel, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            className={({ isActive }) =>
              clsx(
                'group relative flex items-center gap-2.5 px-3 py-[0.4375rem] rounded-lg text-[0.8125rem] font-medium transition-all duration-200',
                isActive
                  ? 'text-brand-300'
                  : 'text-slate-500 hover:text-slate-200 hover:bg-white/[0.03]',
              )
            }
          >
            {({ isActive }) => (
              <>
                {isActive && (
                  <div
                    className="absolute inset-0 rounded-lg"
                    style={{
                      background: 'rgba(16,185,129,0.07)',
                      borderLeft: '2px solid #10b981',
                    }}
                  />
                )}
                <Icon
                  className={clsx(
                    'w-[1.125rem] h-[1.125rem] shrink-0 transition-colors duration-150 relative z-10',
                    isActive ? 'text-brand-400' : 'text-slate-600 group-hover:text-slate-400',
                  )}
                />
                <span className="relative z-10 tracking-[0.01em]">{itemLabel}</span>
              </>
            )}
          </NavLink>
        ))}
      </div>
    </div>
  )
}

export function Sidebar() {
  const location = useLocation()

  // Determine which section is active for subtle visual feedback
  const isGovernanceActive = GOVERNANCE_ITEMS.some(item =>
    location.pathname.startsWith(item.to)
  )
  const isSettingsActive = location.pathname.startsWith('/settings')

  void isGovernanceActive
  void isSettingsActive

  return (
    <aside
      className="w-[15rem] h-screen flex flex-col shrink-0"
      style={{
        background: 'rgba(6, 10, 19, 0.88)',
        backdropFilter: 'blur(20px) saturate(160%)',
        WebkitBackdropFilter: 'blur(20px) saturate(160%)',
        borderRight: '1px solid rgba(148,163,184,0.07)',
      }}
    >
      {/* Brand */}
      <div
        className="px-5 py-4"
        style={{ borderBottom: '1px solid rgba(148,163,184,0.07)' }}
      >
        <div className="flex items-center gap-2.5">
          <div
            className="flex items-center justify-center w-8 h-8 rounded-lg shrink-0"
            style={{
              background: 'linear-gradient(135deg, #10b981 0%, #047857 100%)',
              boxShadow: '0 0 16px rgba(16,185,129,0.28), 0 2px 8px rgba(0,0,0,0.3)',
            }}
          >
            <Shield className="w-4 h-4 text-white" />
          </div>
          <div>
            <div className="font-display text-sm font-bold text-white tracking-tight">Govrix</div>
            <div className="text-[0.625rem] text-slate-500 font-medium tracking-widest uppercase" style={{ fontFamily: 'JetBrains Mono' }}>
              Enterprise
            </div>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 overflow-y-auto py-1">
        <NavSection label="Observability" items={OBSERVABILITY_ITEMS} />
        <NavSection label="Governance" items={GOVERNANCE_ITEMS} />
      </nav>

      {/* Footer */}
      <div style={{ borderTop: '1px solid rgba(148,163,184,0.07)' }}>
        <div className="px-2 py-2">
          <NavLink
            to="/settings"
            className={({ isActive }) =>
              clsx(
                'group relative flex items-center gap-2.5 px-3 py-[0.4375rem] rounded-lg text-[0.8125rem] font-medium transition-all duration-200',
                isActive
                  ? 'text-brand-300'
                  : 'text-slate-500 hover:text-slate-200 hover:bg-white/[0.03]',
              )
            }
          >
            {({ isActive }) => (
              <>
                {isActive && (
                  <div
                    className="absolute inset-0 rounded-lg"
                    style={{
                      background: 'rgba(16,185,129,0.07)',
                      borderLeft: '2px solid #10b981',
                    }}
                  />
                )}
                <Settings
                  className={clsx(
                    'w-[1.125rem] h-[1.125rem] shrink-0 transition-colors relative z-10',
                    isActive ? 'text-brand-400' : 'text-slate-600 group-hover:text-slate-400',
                  )}
                />
                <span className="relative z-10">Settings</span>
              </>
            )}
          </NavLink>
        </div>
        <div
          className="px-5 py-3 text-[0.625rem] text-slate-700 tracking-wider"
          style={{ fontFamily: 'JetBrains Mono' }}
        >
          v0.1.0 · Platform
        </div>
      </div>
    </aside>
  )
}
