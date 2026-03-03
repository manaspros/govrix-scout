import { useState } from 'react'
import { useReports, useReportTypes, useGenerateReport } from '../api/hooks'
import StatusBadge from '../components/common/StatusBadge'
import LoadingState from '../components/common/LoadingState'

export default function ReportsPage() {
  const { data: types } = useReportTypes()
  const { data: reports, isLoading } = useReports()
  const generate = useGenerateReport()
  const [selectedType, setSelectedType] = useState('')
  const [format, setFormat] = useState<'pdf' | 'json' | 'csv'>('pdf')

  if (isLoading) return <LoadingState />

  return (
    <div className="space-y-6">
      <div className="card">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Generate Report</h2>
        <div className="flex gap-3 flex-wrap">
          <select
            value={selectedType}
            onChange={e => setSelectedType(e.target.value)}
            className="flex-1 min-w-[180px] text-sm bg-white dark:bg-[#0d0d1a] border border-slate-200 dark:border-[#272737] rounded-lg px-3 py-2 text-slate-700 dark:text-slate-300"
          >
            <option value="">Select report type…</option>
            {types?.data.map(t => <option key={t.id} value={t.id}>{t.name}</option>)}
          </select>
          <select
            value={format}
            onChange={e => setFormat(e.target.value as typeof format)}
            className="text-sm bg-white dark:bg-[#0d0d1a] border border-slate-200 dark:border-[#272737] rounded-lg px-3 py-2 text-slate-700 dark:text-slate-300"
          >
            <option value="pdf">PDF</option>
            <option value="json">JSON</option>
            <option value="csv">CSV</option>
          </select>
          <button
            onClick={() => { if (selectedType) generate.mutate({ report_type: selectedType, format }) }}
            disabled={!selectedType || generate.isPending}
            className="btn-primary disabled:opacity-40"
          >
            {generate.isPending ? 'Generating…' : 'Generate'}
          </button>
        </div>
      </div>

      <div className="card">
        <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">
          Generated Reports <span className="text-slate-400 font-normal ml-1">({reports?.total ?? 0})</span>
        </h2>
        {!reports?.data.length ? (
          <p className="text-sm text-slate-400 text-center py-8">No reports generated yet</p>
        ) : (
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                {['ID', 'Type', 'Status', 'Created', 'Download'].map(h => (
                  <th key={h} className="pb-2 px-3 font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50 dark:divide-slate-800/30">
              {reports.data.map(r => (
                <tr key={r.id} className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50">
                  <td className="py-2.5 px-3 font-mono text-xs text-slate-500">{r.id.slice(0, 8)}</td>
                  <td className="py-2.5 px-3 text-slate-600 dark:text-slate-400">{r.report_type}</td>
                  <td className="py-2.5 px-3"><StatusBadge status={r.status} /></td>
                  <td className="py-2.5 px-3 text-xs text-slate-400">{new Date(r.created_at).toLocaleString()}</td>
                  <td className="py-2.5 px-3">
                    {r.download_url
                      ? <a href={r.download_url} className="text-xs text-indigo-500 hover:underline">Download</a>
                      : <span className="text-xs text-slate-400">—</span>}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  )
}
