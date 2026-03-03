import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { AuthProvider } from './context/AuthContext'
import Layout from './components/layout/Layout'
import OverviewPage from './pages/OverviewPage'
import EventsPage from './pages/EventsPage'
import AgentsPage from './pages/AgentsPage'
import CostsPage from './pages/CostsPage'
import ReportsPage from './pages/ReportsPage'

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<Navigate to="/overview" replace />} />
            <Route path="overview" element={<OverviewPage />} />
            <Route path="events" element={<EventsPage />} />
            <Route path="agents" element={<AgentsPage />} />
            <Route path="costs" element={<CostsPage />} />
            <Route path="reports" element={<ReportsPage />} />
            <Route path="*" element={<Navigate to="/overview" replace />} />
          </Route>
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  )
}
