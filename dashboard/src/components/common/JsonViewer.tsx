import { useState } from 'react'
import { ChevronDown, ChevronRight } from 'lucide-react'

interface JsonViewerProps {
  data: Record<string, unknown> | unknown
  collapsed?: boolean
  maxDepth?: number
}

interface JsonNodeProps {
  value: unknown
  depth: number
  maxDepth: number
  label?: string
}

function JsonNode({ value, depth, maxDepth, label }: JsonNodeProps) {
  const [open, setOpen] = useState(depth < 2)

  if (value === null) {
    return (
      <span>
        {label !== undefined && <span className="text-slate-400">{label}: </span>}
        <span className="text-slate-400">null</span>
      </span>
    )
  }

  if (typeof value === 'boolean') {
    return (
      <span>
        {label !== undefined && <span className="text-slate-400">{label}: </span>}
        <span className="text-orange-400">{value ? 'true' : 'false'}</span>
      </span>
    )
  }

  if (typeof value === 'number') {
    return (
      <span>
        {label !== undefined && <span className="text-slate-400">{label}: </span>}
        <span className="text-sky-400">{value}</span>
      </span>
    )
  }

  if (typeof value === 'string') {
    return (
      <span>
        {label !== undefined && <span className="text-slate-400">{label}: </span>}
        <span className="text-emerald-400">"{value}"</span>
      </span>
    )
  }

  if (Array.isArray(value)) {
    if (value.length === 0) {
      return (
        <span>
          {label !== undefined && <span className="text-slate-400">{label}: </span>}
          <span className="text-slate-500">[]</span>
        </span>
      )
    }
    return (
      <span className="block">
        <button
          onClick={() => setOpen(o => !o)}
          className="inline-flex items-center gap-0.5 text-slate-300 hover:text-white transition-colors"
        >
          {open ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
          {label !== undefined && <span className="text-slate-400 ml-0.5">{label}: </span>}
          <span className="text-slate-300">Array({value.length})</span>
        </button>
        {open && depth < maxDepth && (
          <div className="ml-4 border-l border-slate-700 pl-3 mt-1 space-y-0.5">
            {value.map((item, i) => (
              <div key={i} className="text-sm">
                <JsonNode value={item} depth={depth + 1} maxDepth={maxDepth} label={String(i)} />
              </div>
            ))}
          </div>
        )}
      </span>
    )
  }

  if (typeof value === 'object') {
    const keys = Object.keys(value as Record<string, unknown>)
    if (keys.length === 0) {
      return (
        <span>
          {label !== undefined && <span className="text-slate-400">{label}: </span>}
          <span className="text-slate-500">{'{}'}</span>
        </span>
      )
    }
    return (
      <span className="block">
        <button
          onClick={() => setOpen(o => !o)}
          className="inline-flex items-center gap-0.5 text-slate-300 hover:text-white transition-colors"
        >
          {open ? <ChevronDown className="w-3 h-3" /> : <ChevronRight className="w-3 h-3" />}
          {label !== undefined && <span className="text-slate-400 ml-0.5">{label}: </span>}
          <span className="text-slate-300">Object({keys.length})</span>
        </button>
        {open && depth < maxDepth && (
          <div className="ml-4 border-l border-slate-700 pl-3 mt-1 space-y-0.5">
            {keys.map(k => (
              <div key={k} className="text-sm">
                <JsonNode
                  value={(value as Record<string, unknown>)[k]}
                  depth={depth + 1}
                  maxDepth={maxDepth}
                  label={k}
                />
              </div>
            ))}
          </div>
        )}
      </span>
    )
  }

  return (
    <span className="text-slate-300">
      {label !== undefined && <span className="text-slate-400">{label}: </span>}
      {String(value)}
    </span>
  )
}

export function JsonViewer({ data, collapsed = false, maxDepth = 6 }: JsonViewerProps) {
  const [isCollapsed, setIsCollapsed] = useState(collapsed)

  return (
    <div className="rounded-lg bg-slate-900 border border-slate-700 overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 bg-slate-800 border-b border-slate-700">
        <span className="text-xs font-medium text-slate-400 uppercase tracking-wider">Payload</span>
        <button
          onClick={() => setIsCollapsed(c => !c)}
          className="text-xs text-slate-400 hover:text-slate-200 transition-colors"
        >
          {isCollapsed ? 'Expand' : 'Collapse'}
        </button>
      </div>
      {!isCollapsed && (
        <div className="p-3 font-mono text-sm text-slate-300 overflow-x-auto max-h-96 overflow-y-auto">
          <JsonNode value={data} depth={0} maxDepth={maxDepth} />
        </div>
      )}
    </div>
  )
}
