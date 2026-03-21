import { useState } from 'react'
import { FileText, Download, Loader2, CheckCircle, AlertCircle } from 'lucide-react'
import { useReportTypes, useGenerateReport, useReports } from '@/api/hooks'
import { EmptyState } from '@/components/common/EmptyState'
import { format, parseISO } from 'date-fns'

// ── Toast notification ────────────────────────────────────────────────────────

interface ToastProps {
  message: string
  type: 'success' | 'error'
  onClose: () => void
}

function Toast({ message, type, onClose }: ToastProps) {
  return (
    <div
      className={`fixed bottom-6 right-6 flex items-center gap-3 px-4 py-3 rounded-xl shadow-2xl border z-50 ${
        type === 'success'
          ? 'bg-slate-800 border-emerald-600/40 text-emerald-400'
          : 'bg-slate-800 border-red-600/40 text-red-400'
      }`}
    >
      {type === 'success'
        ? <CheckCircle className="w-4 h-4 shrink-0" />
        : <AlertCircle className="w-4 h-4 shrink-0" />}
      <span className="text-sm">{message}</span>
      <button onClick={onClose} className="ml-2 text-slate-400 hover:text-slate-200 text-xs">✕</button>
    </div>
  )
}

// ── Report type card ──────────────────────────────────────────────────────────

interface ReportTypeInfo {
  id: string
  name: string
  description: string
  format: 'pdf' | 'json' | 'csv'
}

interface ReportTypeCardProps {
  reportType: ReportTypeInfo
  onGenerate: (id: string) => void
  isGenerating: boolean
}

const FORMAT_COLORS: Record<string, string> = {
  pdf:  'bg-red-500/20 text-red-400 ring-1 ring-red-500/30',
  json: 'bg-brand-500/20 text-brand-400 ring-1 ring-brand-500/30',
  csv:  'bg-emerald-500/20 text-emerald-400 ring-1 ring-emerald-500/30',
}

function ReportTypeCard({ reportType, onGenerate, isGenerating }: ReportTypeCardProps) {
  return (
    <div className="bg-slate-800 rounded-xl border border-slate-700/60 p-5 flex items-start gap-4">
      <div className="flex items-center justify-center w-10 h-10 rounded-lg bg-slate-700/60 shrink-0">
        <FileText className="w-5 h-5 text-slate-300" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-sm font-semibold text-slate-100">{reportType.name}</span>
          <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase ${FORMAT_COLORS[reportType.format] ?? FORMAT_COLORS['json']}`}>
            {reportType.format}
          </span>
        </div>
        <p className="text-xs text-slate-400 mb-3">{reportType.description}</p>
        <button
          onClick={() => onGenerate(reportType.id)}
          disabled={isGenerating}
          className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium bg-brand-600/20 text-brand-400 ring-1 ring-brand-600/30 rounded-lg hover:bg-brand-600/30 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {isGenerating ? (
            <><Loader2 className="w-3 h-3 animate-spin" /> Generating…</>
          ) : (
            <>Generate</>
          )}
        </button>
      </div>
    </div>
  )
}

// ── Reports page ──────────────────────────────────────────────────────────────

export function ReportsPage() {
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' } | null>(null)
  const [generatingId, setGeneratingId] = useState<string | null>(null)

  const { data: typesData, isLoading: typesLoading } = useReportTypes()
  const { data: reportsData } = useReports()
  const generateMutation = useGenerateReport()

  async function handleGenerate(reportTypeId: string) {
    setGeneratingId(reportTypeId)
    try {
      await generateMutation.mutateAsync(reportTypeId)
      setToast({ message: 'Report generation started', type: 'success' })
    } catch {
      setToast({ message: 'Failed to generate report', type: 'error' })
    } finally {
      setGeneratingId(null)
    }
  }

  const reportTypes = typesData?.types ?? []
  const reports = reportsData?.reports ?? []

  return (
    <div className="space-y-6">
      {/* Status bar */}
      <div className="flex items-start gap-3 p-4 bg-emerald-600/10 border border-emerald-600/30 rounded-xl">
        <FileText className="w-4 h-4 text-emerald-400 mt-0.5 shrink-0" />
        <div className="text-xs text-slate-300">
          <span className="font-semibold text-emerald-400">Reports Ready</span>{' '}
          Generate compliance, cost, and security reports. Download as PDF, JSON, or CSV when complete.
        </div>
      </div>

      {/* Report types */}
      <div>
        <h2 className="text-sm font-semibold text-slate-200 mb-3">Available Report Types</h2>
        {typesLoading ? (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="animate-pulse bg-slate-800 rounded-xl h-28 border border-slate-700/60" />
            ))}
          </div>
        ) : reportTypes.length === 0 ? (
          <EmptyState
            icon={FileText}
            title="No report types available"
            description="Report types will be available when the API is connected."
          />
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {reportTypes.map(rt => (
              <ReportTypeCard
                key={rt.id}
                reportType={rt}
                onGenerate={id => void handleGenerate(id)}
                isGenerating={generatingId === rt.id}
              />
            ))}
          </div>
        )}
      </div>

      {/* Previous reports */}
      {reports.length > 0 && (
        <div>
          <h2 className="text-sm font-semibold text-slate-200 mb-3">Generated Reports</h2>
          <div className="overflow-x-auto rounded-xl border border-slate-700">
            <table className="w-full text-sm">
              <thead className="bg-slate-800/80">
                <tr>
                  {['ID', 'Type', 'Status', 'Created', 'Download'].map(h => (
                    <th key={h} className="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-700/40">
                {reports.map(report => (
                  <tr key={report.id} className="hover:bg-slate-700/20 transition-colors">
                    <td className="px-4 py-3 text-xs text-slate-400 font-mono">{report.id.slice(0, 16)}…</td>
                    <td className="px-4 py-3 text-xs text-slate-300">{report.report_type}</td>
                    <td className="px-4 py-3">
                      <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                        report.status === 'ready' ? 'bg-emerald-500/20 text-emerald-400' :
                        report.status === 'error' ? 'bg-red-500/20 text-red-400' :
                        'bg-yellow-500/20 text-yellow-400'
                      }`}>
                        {report.status}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-xs text-slate-400">
                      {format(parseISO(report.created_at), 'MMM d, HH:mm')}
                    </td>
                    <td className="px-4 py-3">
                      {report.status === 'ready' ? (() => {
                        const downloadUrl = report.download_url || `/api/v1/reports/${report.id}/download`;
                        return (
                          <a
                            href={downloadUrl}
                            className="inline-flex items-center gap-1 text-xs text-brand-400 hover:text-brand-300"
                          >
                            <Download className="w-3.5 h-3.5" />
                            Download
                          </a>
                        );
                      })() : (
                        <span className="text-xs text-slate-600">
                          {report.status === 'generating' ? 'Processing…' : 'N/A'}
                        </span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Toast */}
      {toast && (
        <Toast
          message={toast.message}
          type={toast.type}
          onClose={() => setToast(null)}
        />
      )}
    </div>
  )
}
