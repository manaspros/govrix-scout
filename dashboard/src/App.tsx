import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { Layout } from '@/components/layout/Layout'
import { OverviewPage } from '@/pages/OverviewPage'
import { AgentsPage } from '@/pages/AgentsPage'
import { EventsPage } from '@/pages/EventsPage'
import { CostsPage } from '@/pages/CostsPage'
import { ReportsPage } from '@/pages/ReportsPage'
import { SettingsPage } from '@/pages/SettingsPage'

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
          {/* Redirect root to /overview */}
          <Route path="/" element={<Navigate to="/overview" replace />} />

          {/* Main layout with sidebar */}
          <Route element={<Layout />}>
            <Route path="/overview" element={<OverviewPage />} />
            <Route path="/agents"   element={<AgentsPage />} />
            <Route path="/events"   element={<EventsPage />} />
            <Route path="/costs"    element={<CostsPage />} />
            <Route path="/reports"  element={<ReportsPage />} />
            <Route path="/settings" element={<SettingsPage />} />
          </Route>

          {/* Catch-all 404 */}
          <Route path="*" element={<Navigate to="/overview" replace />} />
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  )
}
