import { useState } from 'react'
import { FileText, Download, Loader2, CheckCircle2, AlertCircle, FileDown } from 'lucide-react'
import { useReportTypes, useReports } from '../api/hooks'
import * as apiClient from '../api/client'

/* ── Helpers ──────────────────────────────────────────────────── */
const toCSV = (rows: Record<string, unknown>[]): string => {
  if (!rows || rows.length === 0) return ''
  const keys = Object.keys(rows[0])
  const escape = (v: unknown): string => {
    if (v == null) return ''
    const s = String(v).replace(/"/g, '""')
    return s.includes(',') || s.includes('\n') || s.includes('"') ? `"${s}"` : s
  }
  return [keys.join(','), ...rows.map(r => keys.map(k => escape(r[k])).join(','))].join('\n')
}

const downloadFile = (content: string, fileName: string, mime: string): void => {
  const blob = new Blob([content], { type: mime })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = fileName
  a.click()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

const fmtDate = (): string => new Date().toISOString().slice(0, 10)

/* ── Report template definitions ──────────────────────────────── */
interface ReportTemplate {
  id: string
  icon: string
  label: string
  desc: string
  color: string
  bg: string
  fetch: () => Promise<unknown>
}

const TEMPLATES: ReportTemplate[] = [
  {
    id: 'usage_summary',
    icon: '📊',
    label: 'Usage Summary',
    desc: 'Event counts, tokens, latency overview',
    color: 'text-indigo-500',
    bg: 'bg-indigo-50',
    fetch: async () => {
      const [events, costs] = await Promise.all([
        apiClient.getEvents({ limit: 1000 }),
        apiClient.getCostSummary(),
      ])
      const evList = events?.data ?? []
      const cs = costs ?? {}
      return {
        generated_at: new Date().toISOString(),
        total_events: evList.length,
        total_requests: cs.total_requests ?? evList.length,
        total_input_tokens: cs.total_input_tokens ?? 0,
        total_output_tokens: cs.total_output_tokens ?? 0,
        total_cost_usd: cs.total_cost_usd ?? 0,
        models: [...new Set(evList.map((e) => e.model).filter(Boolean))],
        agents: [...new Set(evList.map((e) => e.agent_id).filter(Boolean))],
        events: evList.slice(0, 500).map(e => ({
          timestamp: e.timestamp,
          agent_id: e.agent_id,
          model: e.model,
          input_tokens: e.input_tokens,
          output_tokens: e.output_tokens,
          cost_usd: e.cost_usd,
          status_code: e.status_code,
          latency_ms: e.latency_ms,
        })),
      }
    },
  },
  {
    id: 'cost_breakdown',
    icon: '💰',
    label: 'Cost Breakdown',
    desc: 'Spend by model and agent',
    color: 'text-emerald-500',
    bg: 'bg-emerald-50',
    fetch: async () => {
      const [breakdown, summary] = await Promise.all([
        apiClient.getCostBreakdown(),
        apiClient.getCostSummary(),
      ])
      return {
        generated_at: new Date().toISOString(),
        summary: summary ?? {},
        by_model: (breakdown?.by_model ?? []).map(r => ({
          model: r.label,
          total_cost_usd: r.cost_usd,
          request_count: r.requests,
          input_tokens: r.input_tokens,
          output_tokens: r.output_tokens,
        })),
        by_agent: (breakdown?.by_agent ?? []).map(r => ({
          agent: r.label,
          total_cost_usd: r.cost_usd,
          request_count: r.requests,
        })),
        by_provider: (breakdown?.by_provider ?? []).map(r => ({
          provider: r.label,
          total_cost_usd: r.cost_usd,
          request_count: r.requests,
        })),
      }
    },
  },
  {
    id: 'agent_inventory',
    icon: '🤖',
    label: 'Agent Inventory',
    desc: 'All registered agents and stats',
    color: 'text-violet-500',
    bg: 'bg-violet-50',
    fetch: async () => {
      const agents = await apiClient.getAgents()
      const list = agents?.data ?? []
      return {
        generated_at: new Date().toISOString(),
        total_agents: list.length,
        agents: list.map(a => ({
          id: a.id,
          name: a.name,
          status: a.status,
          total_requests: a.total_requests,
          total_cost_usd: a.total_cost_usd,
          total_input_tokens: a.total_input_tokens,
          total_output_tokens: a.total_output_tokens,
          first_seen: a.first_seen,
          last_seen: a.last_seen,
        })),
      }
    },
  },
  {
    id: 'activity_log',
    icon: '📋',
    label: 'Activity Log',
    desc: 'Full event log export (last 1000)',
    color: 'text-sky-500',
    bg: 'bg-sky-50',
    fetch: async () => {
      const events = await apiClient.getEvents({ limit: 1000, offset: 0 })
      const list = events?.data ?? []
      return list.map(e => ({
        timestamp: e.timestamp,
        agent_id: e.agent_id,
        session_id: e.session_id,
        model: e.model,
        provider: e.provider,
        kind: e.kind,
        protocol: e.protocol,
        status_code: e.status_code,
        input_tokens: e.input_tokens,
        output_tokens: e.output_tokens,
        cost_usd: e.cost_usd,
        latency_ms: e.latency_ms,
        pii_detected: e.pii_detected,
        compliance_tag: e.compliance_tag,
      }))
    },
  },
]

interface HistoryEntry {
  id: string
  type: string
  label: string
  format: string
  created_at: string
  fileName: string
  data: unknown
}

interface ToastState {
  text: string
  ok: boolean
}

/* ── Component ────────────────────────────────────────────────── */
export default function ReportsPage() {
  const [generating, setGenerating] = useState<Record<string, boolean>>({})
  const [history, setHistory] = useState<HistoryEntry[]>([])
  const [toast, setToast] = useState<ToastState | null>(null)

  // Scout API hooks (wired, though client-side generation is primary for OSS templates)
  const { data: _reportTypes } = useReportTypes()
  const { data: reports } = useReports()

  const showToast = (msg: string, ok = true): void => {
    setToast({ text: msg, ok })
    setTimeout(() => setToast(null), 4000)
  }

  const handleGenerate = async (template: ReportTemplate, format = 'json'): Promise<void> => {
    const key = `${template.id}-${format}`
    setGenerating(g => ({ ...g, [key]: true }))
    try {
      const data = await template.fetch()
      const dateStr = fmtDate()
      let fileName: string

      if (format === 'csv') {
        const rows = Array.isArray(data)
          ? (data as Record<string, unknown>[])
          : ((data as Record<string, unknown[]>).events ??
             (data as Record<string, unknown[]>).agents ??
             (data as Record<string, unknown[]>).by_model ??
             [data as Record<string, unknown>])
        fileName = `govrix-${template.id}-${dateStr}.csv`
        downloadFile(toCSV(rows as Record<string, unknown>[]), fileName, 'text/csv')
      } else {
        fileName = `govrix-${template.id}-${dateStr}.json`
        downloadFile(JSON.stringify(data, null, 2), fileName, 'application/json')
      }

      const entry: HistoryEntry = {
        id: `${template.id}-${Date.now()}`,
        type: template.id,
        label: template.label,
        format: format.toUpperCase(),
        created_at: new Date().toISOString(),
        fileName,
        data,
      }
      setHistory(h => [entry, ...h])
      showToast(`"${template.label}" downloaded as ${fileName}`, true)
    } catch (e) {
      showToast(
        `Failed to generate ${template.label}: ${e instanceof Error ? e.message : String(e)}`,
        false,
      )
    } finally {
      setGenerating(g => ({ ...g, [key]: false }))
    }
  }

  const handleReDownload = (entry: HistoryEntry): void => {
    if (entry.format === 'CSV') {
      const data = entry.data
      const rows = Array.isArray(data)
        ? (data as Record<string, unknown>[])
        : ((data as Record<string, unknown[]>).events ??
           (data as Record<string, unknown[]>).agents ??
           (data as Record<string, unknown[]>).by_model ??
           [data as Record<string, unknown>])
      downloadFile(toCSV(rows as Record<string, unknown>[]), entry.fileName, 'text/csv')
    } else {
      downloadFile(JSON.stringify(entry.data, null, 2), entry.fileName, 'application/json')
    }
    showToast(`Re-downloaded ${entry.fileName}`, true)
  }

  // Combine local history + server-side reports from useReports()
  const serverReports = reports?.data ?? []

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-[1400px] mx-auto space-y-6">

        {/* Toast */}
        {toast && (
          <div className={`fixed top-4 right-4 z-50 flex items-center gap-2 px-4 py-3 rounded-lg shadow-xl text-sm font-medium border-l-4 ${
            toast.ok
              ? 'bg-green-50 border-green-500 text-green-700'
              : 'bg-red-50 border-red-500 text-red-700'
          }`}>
            {toast.ok
              ? <CheckCircle2 className="w-4 h-4 shrink-0" />
              : <AlertCircle className="w-4 h-4 shrink-0" />}
            {toast.text}
          </div>
        )}

        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-bold text-slate-900">Reports</h2>
            <p className="text-xs text-slate-400 mt-0.5">
              Generate and download governance reports — runs directly in your browser
            </p>
          </div>
        </div>

        {/* OSS Templates */}
        <div>
          <h3 className="text-[10px] uppercase tracking-widest text-slate-400 font-bold mb-3">
            Scout Reports
          </h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
            {TEMPLATES.map(t => {
              const busyJSON = generating[`${t.id}-json`]
              const busyCSV = generating[`${t.id}-csv`]
              const anyBusy = busyJSON || busyCSV
              return (
                <div key={t.id} className="stat-card flex flex-col gap-3">
                  <div className="flex items-start gap-3">
                    <div className={`w-9 h-9 rounded-lg ${t.bg} flex items-center justify-center text-lg shrink-0`}>
                      {t.icon}
                    </div>
                    <div className="min-w-0">
                      <p className="text-sm font-bold text-slate-800 leading-tight">{t.label}</p>
                      <p className="text-[10px] text-slate-400 mt-0.5 leading-snug">{t.desc}</p>
                    </div>
                  </div>
                  {/* JSON — prominent */}
                  <button
                    onClick={() => handleGenerate(t, 'json')}
                    disabled={anyBusy}
                    className="w-full btn-primary flex items-center justify-center gap-1.5 text-[11px] py-2 disabled:opacity-60"
                  >
                    {busyJSON
                      ? <Loader2 className="w-3.5 h-3.5 animate-spin" />
                      : <FileDown className="w-3.5 h-3.5" />}
                    Export JSON
                  </button>
                  {/* CSV — secondary */}
                  <div className="flex gap-2">
                    <button
                      onClick={() => handleGenerate(t, 'csv')}
                      disabled={anyBusy}
                      className="btn-secondary flex-1 flex items-center justify-center gap-1 text-[10px] py-1.5 disabled:opacity-60"
                    >
                      {busyCSV
                        ? <Loader2 className="w-3 h-3 animate-spin" />
                        : <Download className="w-3 h-3" />}
                      CSV
                    </button>
                  </div>
                </div>
              )
            })}
          </div>
        </div>

        {/* Generated This Session (local history) */}
        <div className="bg-white border border-slate-200 rounded-xl overflow-hidden">
          <div className="px-5 py-3 border-b border-slate-100 bg-slate-50/50 flex items-center justify-between">
            <h3 className="text-sm font-bold text-slate-700">Generated This Session</h3>
            <span className="text-[10px] text-slate-400 metric-font">
              {history.length} file{history.length !== 1 ? 's' : ''}
            </span>
          </div>
          <table className="w-full">
            <thead>
              <tr className="border-b border-slate-100">
                <th className="table-header text-left py-3 px-5">Report</th>
                <th className="table-header text-left py-3 px-5">Filename</th>
                <th className="table-header text-left py-3 px-5">Generated</th>
                <th className="table-header text-left py-3 px-5">Format</th>
                <th className="table-header text-right py-3 px-5">Actions</th>
              </tr>
            </thead>
            <tbody>
              {history.map(r => (
                <tr
                  key={r.id}
                  className="border-b border-slate-50 hover:bg-slate-50/50 transition-colors"
                >
                  <td className="table-cell font-semibold">{r.label}</td>
                  <td className="table-cell text-xs text-slate-400 metric-font">{r.fileName}</td>
                  <td className="table-cell text-xs text-slate-400">
                    {new Date(r.created_at).toLocaleTimeString()}
                  </td>
                  <td className="table-cell">
                    <span className="badge badge-neutral">{r.format}</span>
                  </td>
                  <td className="table-cell text-right">
                    <button
                      onClick={() => handleReDownload(r)}
                      className="btn-secondary text-[10px] py-1 px-2 inline-flex items-center gap-1"
                    >
                      <Download className="w-3 h-3" /> Download
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
          {history.length === 0 && (
            <div className="text-center py-12 text-slate-400">
              <FileText className="w-10 h-10 mx-auto mb-3 text-slate-300" />
              <p className="text-sm font-medium">No reports generated yet</p>
              <p className="text-xs mt-1">Click JSON or CSV on any template above</p>
            </div>
          )}
        </div>

        {/* Server-side report history (from useReports hook) */}
        {serverReports.length > 0 && (
          <div className="bg-white border border-slate-200 rounded-xl overflow-hidden">
            <div className="px-5 py-3 border-b border-slate-100 bg-slate-50/50 flex items-center justify-between">
              <h3 className="text-sm font-bold text-slate-700">Server Reports</h3>
              <span className="text-[10px] text-slate-400 metric-font">
                {serverReports.length} report{serverReports.length !== 1 ? 's' : ''}
              </span>
            </div>
            <table className="w-full">
              <thead>
                <tr className="border-b border-slate-100">
                  <th className="table-header text-left py-3 px-5">ID</th>
                  <th className="table-header text-left py-3 px-5">Type</th>
                  <th className="table-header text-left py-3 px-5">Status</th>
                  <th className="table-header text-left py-3 px-5">Created</th>
                  <th className="table-header text-right py-3 px-5">Download</th>
                </tr>
              </thead>
              <tbody>
                {serverReports.map(r => (
                  <tr
                    key={r.id}
                    className="border-b border-slate-50 hover:bg-slate-50/50 transition-colors"
                  >
                    <td className="table-cell text-xs metric-font text-slate-500">
                      {r.id.slice(0, 8)}
                    </td>
                    <td className="table-cell text-xs text-slate-600">{r.report_type}</td>
                    <td className="table-cell">
                      <span className={`badge ${
                        r.status === 'complete'
                          ? 'badge-success'
                          : r.status === 'failed'
                          ? 'badge-danger'
                          : 'badge-neutral'
                      }`}>
                        {r.status}
                      </span>
                    </td>
                    <td className="table-cell text-xs text-slate-400">
                      {new Date(r.created_at).toLocaleString()}
                    </td>
                    <td className="table-cell text-right">
                      {r.download_url ? (
                        <a
                          href={r.download_url}
                          className="btn-secondary text-[10px] py-1 px-2 inline-flex items-center gap-1"
                        >
                          <Download className="w-3 h-3" /> Download
                        </a>
                      ) : (
                        <span className="text-xs text-slate-400">—</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

      </div>
    </div>
  )
}
