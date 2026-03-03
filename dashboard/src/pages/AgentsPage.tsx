import React from 'react'
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts'
import { Bot, RefreshCw } from 'lucide-react'
import { useAgents, useRetireAgent } from '../api/hooks'

const fmtNum = (n: number | undefined | null): string =>
  typeof n === 'number' ? n.toLocaleString() : '0'

const fmtUsd = (n: number | undefined | null): string =>
  typeof n === 'number' ? `$${n.toFixed(4)}` : '$0'

const statusColor = (s: string | undefined): string => {
  const map: Record<string, string> = {
    active: 'badge-success',
    idle: 'badge-neutral',
    error: 'badge-danger',
    blocked: 'badge-danger',
    retired: 'badge-neutral',
  }
  return map[s ?? ''] || 'badge-neutral'
}

export default function AgentsPage() {
  const { data, isLoading, refetch } = useAgents()
  const retireAgent = useRetireAgent()

  const agents = data?.data ?? []

  // Bar chart data — top agents by requests
  const chartData = [...agents]
    .sort((a, b) => (b.total_requests || 0) - (a.total_requests || 0))
    .slice(0, 10)
    .map(a => ({
      name: a.id?.slice(0, 16) || 'unknown',
      requests: a.total_requests || 0,
      cost: a.total_cost_usd || 0,
    }))

  return (
    <div className="flex-1 overflow-y-auto p-6">
      <div className="max-w-[1400px] mx-auto space-y-4">

        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-lg font-bold text-slate-900">Agents</h2>
            <p className="text-xs text-slate-400">
              {agents.length} agent{agents.length !== 1 ? 's' : ''} discovered
            </p>
          </div>
          <button
            onClick={() => refetch()}
            className="btn-secondary flex items-center gap-1.5 text-xs"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isLoading ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        </div>

        {/* Top Agents Chart */}
        {chartData.length > 0 && (
          <div className="bg-white border border-slate-200 rounded-xl p-5">
            <h3 className="text-sm font-bold text-slate-700 mb-4">Top Agents by Requests</h3>
            <ResponsiveContainer width="100%" height={200}>
              <BarChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" stroke="#f1f5f9" />
                <XAxis
                  dataKey="name"
                  tick={{ fontSize: 10, fill: '#94a3b8' }}
                  axisLine={false}
                  tickLine={false}
                />
                <YAxis
                  tick={{ fontSize: 10, fill: '#94a3b8' }}
                  axisLine={false}
                  tickLine={false}
                  width={50}
                />
                <Tooltip contentStyle={{ fontSize: 11, borderRadius: 8 }} />
                <Bar dataKey="requests" fill="#6366f1" radius={[4, 4, 0, 0]} name="Requests" />
              </BarChart>
            </ResponsiveContainer>
          </div>
        )}

        {/* Agent Table */}
        <div className="bg-white border border-slate-200 rounded-xl overflow-hidden">
          <table className="w-full">
            <thead>
              <tr className="border-b border-slate-100 bg-slate-50/50">
                <th className="table-header text-left py-3 px-4">Agent ID</th>
                <th className="table-header text-left py-3 px-4">Name</th>
                <th className="table-header text-center py-3 px-4">Status</th>
                <th className="table-header text-right py-3 px-4">Requests</th>
                <th className="table-header text-right py-3 px-4">Tokens In</th>
                <th className="table-header text-right py-3 px-4">Tokens Out</th>
                <th className="table-header text-right py-3 px-4">Cost</th>
                <th className="table-header text-left py-3 px-4">Last Seen</th>
                <th className="table-header text-center py-3 px-4">Actions</th>
              </tr>
            </thead>
            <tbody>
              {agents.map((a, i) => (
                <tr
                  key={a.id || i}
                  className="border-b border-slate-50 hover:bg-slate-50/50 transition-colors"
                >
                  <td className="table-cell text-xs font-medium text-primary max-w-[180px] truncate">
                    {a.id || '—'}
                  </td>
                  <td className="table-cell text-xs text-slate-500">{a.name || '—'}</td>
                  <td className="table-cell text-center">
                    <span className={`badge ${statusColor(a.status)}`}>
                      {a.status || 'unknown'}
                    </span>
                  </td>
                  <td className="table-cell text-xs metric-font text-right">
                    {fmtNum(a.total_requests)}
                  </td>
                  <td className="table-cell text-xs metric-font text-right">
                    {fmtNum(a.total_input_tokens)}
                  </td>
                  <td className="table-cell text-xs metric-font text-right">
                    {fmtNum(a.total_output_tokens)}
                  </td>
                  <td className="table-cell text-xs metric-font text-right text-slate-600">
                    {fmtUsd(a.total_cost_usd)}
                  </td>
                  <td className="table-cell text-xs text-slate-400">
                    {a.last_seen
                      ? new Date(a.last_seen).toLocaleString([], {
                          month: 'short',
                          day: 'numeric',
                          hour: '2-digit',
                          minute: '2-digit',
                        })
                      : '—'}
                  </td>
                  <td className="table-cell text-center">
                    {a.status === 'active' && (
                      <button
                        onClick={() => retireAgent.mutate(a.id)}
                        disabled={retireAgent.isPending}
                        className="text-xs text-red-500 hover:text-red-700 font-medium disabled:opacity-40 transition-colors"
                      >
                        Retire
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>

          {agents.length === 0 && !isLoading && (
            <div className="text-center py-12 text-slate-400">
              <Bot className="w-10 h-10 mx-auto mb-3 text-slate-300" />
              <p className="text-sm font-medium">No agents discovered yet</p>
              <p className="text-xs mt-1">
                Agents are auto-registered when they send requests through the proxy
              </p>
            </div>
          )}
        </div>

      </div>
    </div>
  )
}
