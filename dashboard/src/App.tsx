import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { Layout } from '@/components/layout/Layout'
import { OverviewPage } from '@/pages/OverviewPage'
import { AgentsPage } from '@/pages/AgentsPage'
import { AgentDetailPage } from '@/pages/AgentDetailPage'
import { EventsPage } from '@/pages/EventsPage'
import { CostsPage } from '@/pages/CostsPage'
import { ReportsPage } from '@/pages/ReportsPage'
import { SettingsPage } from '@/pages/SettingsPage'
import { RiskOverviewPage } from '@/pages/RiskOverviewPage'
import { PoliciesPage } from '@/pages/PoliciesPage'
import { CompliancePage } from '@/pages/CompliancePage'
import { SessionsPage } from '@/pages/SessionsPage'
import { BudgetPage } from '@/pages/BudgetPage'
import { KillSwitchPage } from '@/pages/KillSwitchPage'
import { PiiActivityPage } from '@/pages/PiiActivityPage'
import { EuAiActPage } from '@/pages/EuAiActPage'
import { TraceListPage } from '@/pages/TraceListPage'
import { TracePage } from '@/pages/TracePage'
import { StreamPage } from '@/pages/StreamPage'

// ── Query client ─────────────────────────────────────────────────────────────

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 10_000,
      retry: 2,
      refetchOnWindowFocus: false,
    },
  },
})

// ── App root ──────────────────────────────────────────────────────────────────

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <Routes>
          {/* Redirect root to overview (shows real OSS data) */}
          <Route path="/" element={<Navigate to="/overview" replace />} />

          {/* Main layout with sidebar */}
          <Route element={<Layout />}>
            {/* Observability */}
            <Route path="/overview" element={<OverviewPage />} />
            <Route path="/agents"   element={<AgentsPage />} />
            <Route path="/agents/:agentId" element={<AgentDetailPage />} />
            <Route path="/events"   element={<EventsPage />} />
            <Route path="/costs"    element={<CostsPage />} />
            <Route path="/reports"  element={<ReportsPage />} />
            <Route path="/traces"           element={<TraceListPage />} />
            <Route path="/traces/:traceId"  element={<TracePage />} />
            <Route path="/stream"           element={<StreamPage />} />

            {/* Governance */}
            <Route path="/risk"        element={<RiskOverviewPage />} />
            <Route path="/policies"    element={<PoliciesPage />} />
            <Route path="/compliance"  element={<CompliancePage />} />
            <Route path="/sessions"    element={<SessionsPage />} />
            <Route path="/budget"      element={<BudgetPage />} />
            <Route path="/kill-switch" element={<KillSwitchPage />} />
            <Route path="/pii"         element={<PiiActivityPage />} />
            <Route path="/eu-ai-act"   element={<EuAiActPage />} />

            {/* Settings */}
            <Route path="/settings" element={<SettingsPage />} />
          </Route>

          {/* Catch-all 404 */}
          <Route path="*" element={<Navigate to="/overview" replace />} />
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
