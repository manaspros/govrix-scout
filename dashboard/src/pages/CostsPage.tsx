import { useCostSummary, useCostBreakdown } from '../api/hooks'
import LoadingState from '../components/common/LoadingState'

export default function CostsPage() {
  const { data: summary, isLoading } = useCostSummary()
  const { data: breakdown } = useCostBreakdown()

  if (isLoading) return <LoadingState />

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        {[
          { label: 'Total Cost', value: `$${summary?.total_cost_usd?.toFixed(4) ?? '0'}` },
          { label: 'Total Requests', value: summary?.total_requests?.toLocaleString() ?? '0' },
          { label: 'Avg Cost / Req', value: `$${summary?.avg_cost_per_request?.toFixed(5) ?? '0'}` },
          { label: 'Total Tokens', value: ((summary?.total_input_tokens ?? 0) + (summary?.total_output_tokens ?? 0)).toLocaleString() },
        ].map(({ label, value }) => (
          <div key={label} className="card">
            <p className="text-xs text-slate-500 mb-1">{label}</p>
            <p className="text-xl font-bold text-slate-900 dark:text-slate-100">{value}</p>
          </div>
        ))}
      </div>

      {breakdown?.by_model && breakdown.by_model.length > 0 && (
        <div className="card">
          <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Breakdown by Model</h2>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                {['Model', 'Requests', 'Cost (USD)', 'Input Tokens', 'Output Tokens'].map(h => (
                  <th key={h} className="pb-2 px-3 font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50 dark:divide-slate-800/30">
              {breakdown.by_model.map(row => (
                <tr key={row.label} className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50">
                  <td className="py-2.5 px-3 font-medium text-slate-900 dark:text-slate-100">{row.label}</td>
                  <td className="py-2.5 px-3 text-slate-500">{row.requests.toLocaleString()}</td>
                  <td className="py-2.5 px-3 text-slate-500">${row.cost_usd.toFixed(4)}</td>
                  <td className="py-2.5 px-3 text-slate-500">{row.input_tokens.toLocaleString()}</td>
                  <td className="py-2.5 px-3 text-slate-500">{row.output_tokens.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {breakdown?.by_agent && breakdown.by_agent.length > 0 && (
        <div className="card">
          <h2 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-4">Breakdown by Agent</h2>
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-xs text-slate-500 border-b border-slate-100 dark:border-slate-800">
                {['Agent', 'Requests', 'Cost (USD)'].map(h => (
                  <th key={h} className="pb-2 px-3 font-medium">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-50 dark:divide-slate-800/30">
              {breakdown.by_agent.map(row => (
                <tr key={row.label} className="hover:bg-slate-50 dark:hover:bg-[#1a1a2e]/50">
                  <td className="py-2.5 px-3 font-mono text-xs text-slate-700 dark:text-slate-300">{row.label}</td>
                  <td className="py-2.5 px-3 text-slate-500">{row.requests.toLocaleString()}</td>
                  <td className="py-2.5 px-3 text-slate-500">${row.cost_usd.toFixed(4)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
