import React, { createContext, useContext, useState, useEffect, type ReactNode } from 'react'
import { BrowserRouter, Routes, Route, Navigate, NavLink, Outlet, useLocation } from 'react-router-dom'
import {
  LayoutDashboard, Radio, Bot, DollarSign, FileText,
  ChevronRight, Search, Zap, Moon, Sun,
} from 'lucide-react'
import { useHealth } from './api/hooks'

import OverviewPage from './pages/OverviewPage'
import EventsPage from './pages/EventsPage'
import AgentsPage from './pages/AgentsPage'
import CostsPage from './pages/CostsPage'
import ReportsPage from './pages/ReportsPage'

/* ── Auth Context ──────────────────────────────────────────────── */
interface AuthContextValue {
  darkMode: boolean
  toggleDark: () => void
}

const AuthContext = createContext<AuthContextValue | null>(null)

export const useAuth = (): AuthContextValue => {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used inside AuthProvider')
  return ctx
}

const AuthProvider = ({ children }: { children: ReactNode }) => {
  const [darkMode, setDarkMode] = useState<boolean>(() => {
    try { return localStorage.getItem('govrix-dark') === 'true' } catch { return false }
  })

  useEffect(() => {
    const html = document.documentElement
    if (darkMode) {
      html.classList.add('dark')
    } else {
      html.classList.remove('dark')
    }
    try { localStorage.setItem('govrix-dark', String(darkMode)) } catch {}
  }, [darkMode])

  const toggleDark = () => setDarkMode(d => !d)

  return (
    <AuthContext.Provider value={{ darkMode, toggleDark }}>
      {children}
    </AuthContext.Provider>
  )
}

/* ── Navigation ────────────────────────────────────────────────── */
interface NavItem {
  name: string
  to: string
  icon: React.ComponentType<{ className?: string }>
}

const navItems: NavItem[] = [
  { name: 'Dashboard', to: '/overview', icon: LayoutDashboard },
  { name: 'Events',    to: '/events',   icon: Radio },
  { name: 'Agents',    to: '/agents',   icon: Bot },
  { name: 'Costs',     to: '/costs',    icon: DollarSign },
  { name: 'Reports',   to: '/reports',  icon: FileText },
]

/* ── Sidebar ───────────────────────────────────────────────────── */
const Sidebar = () => {
  const { darkMode } = useAuth()
  const { data: health } = useHealth()

  const dotColor = !health
    ? 'bg-slate-300'
    : health.status === 'ok'
    ? 'bg-emerald-500 animate-pulse'
    : 'bg-amber-500'

  return (
    <aside
      className="w-64 fixed inset-y-0 left-0 glass-panel border-r z-50 flex flex-col"
      style={darkMode ? { backgroundColor: '#0a0e18', borderColor: '#2a3347' } : {}}
    >
      <div className="p-5 border-b border-slate-100">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 bg-primary rounded-lg flex items-center justify-center">
            <Zap className="w-4 h-4 text-white" />
          </div>
          <div>
            <h1 className="text-lg font-bold tracking-tight text-slate-900">
              Govrix<span className="text-primary">.</span>
            </h1>
            <p className="text-[10px] text-slate-400 font-medium -mt-0.5">Scout OSS</p>
          </div>
        </div>
      </div>

      <nav className="flex-1 px-3 py-4 space-y-6 overflow-y-auto">
        <div>
          <div className="space-y-0.5">
            {navItems.map(item => {
              const Icon = item.icon
              return (
                <NavLink
                  key={item.to}
                  to={item.to}
                  className={({ isActive }) => isActive ? 'sidebar-item-active' : 'sidebar-item'}
                >
                  <Icon className="w-[18px] h-[18px]" />
                  <span>{item.name}</span>
                </NavLink>
              )
            })}
          </div>
        </div>
      </nav>

      <div className="p-4 border-t border-slate-100">
        <div className="flex items-center gap-2 px-2">
          <div className={`w-2 h-2 rounded-full ${dotColor}`} />
          <span className="text-[11px] text-slate-400 font-medium metric-font">PROXY ONLINE</span>
        </div>
      </div>
    </aside>
  )
}

/* ── Header ────────────────────────────────────────────────────── */
const Header = () => {
  const { darkMode, toggleDark } = useAuth()
  const location = useLocation()

  const currentPage = navItems.find(i => i.to === location.pathname)

  return (
    <header
      className="h-14 border-b border-slate-200 bg-white sticky top-0 z-40 px-6 flex items-center justify-between"
      style={darkMode ? { backgroundColor: '#161b27', borderColor: '#2a3347' } : {}}
    >
      <div className="flex items-center gap-2 text-sm">
        <span className="text-slate-400 text-xs">Scout OSS</span>
        <ChevronRight className="w-3 h-3 text-slate-300" />
        <span className="font-semibold text-slate-700">{currentPage?.name ?? 'Page'}</span>
      </div>
      <div className="flex items-center gap-4">
        <div className="relative w-56 hidden lg:block">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400 w-3.5 h-3.5" />
          <input
            className="w-full bg-slate-50 border border-slate-200 rounded-lg pl-9 pr-4 py-1.5 text-xs focus:ring-2 focus:ring-primary/30 placeholder:text-slate-400"
            placeholder="Search..."
            type="text"
            readOnly
          />
        </div>
        <button
          onClick={toggleDark}
          title={darkMode ? 'Switch to light mode' : 'Switch to dark mode'}
          className="w-8 h-8 rounded-lg border border-slate-200 flex items-center justify-center text-slate-500 hover:text-primary hover:border-primary/30 transition-colors"
          style={darkMode ? { backgroundColor: '#1c2333', borderColor: '#2a3347', color: '#7c7ffa' } : {}}
        >
          {darkMode ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
        </button>
        <div className="w-8 h-8 rounded-full bg-primary/10 border border-primary/20 flex items-center justify-center">
          <span className="text-primary text-xs font-bold">G</span>
        </div>
      </div>
    </header>
  )
}

/* ── Layout ────────────────────────────────────────────────────── */
const Layout = () => (
  <div className="flex min-h-screen">
    <Sidebar />
    <main className="flex-1 ml-64 min-h-screen flex flex-col bg-slate-50">
      <Header />
      <Outlet />
    </main>
  </div>
)

/* ── App ───────────────────────────────────────────────────────── */
export { AuthProvider }

export default function App() {
  return (
    <AuthProvider>
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<Layout />}>
            <Route index element={<Navigate to="/overview" replace />} />
            <Route path="overview" element={<OverviewPage />} />
            <Route path="events"   element={<EventsPage />} />
            <Route path="agents"   element={<AgentsPage />} />
            <Route path="costs"    element={<CostsPage />} />
            <Route path="reports"  element={<ReportsPage />} />
            <Route path="*"        element={<Navigate to="/overview" replace />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </AuthProvider>
  )
}
