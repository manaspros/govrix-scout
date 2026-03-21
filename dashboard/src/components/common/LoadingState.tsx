interface LoadingStateProps {
  rows?: number
  cols?: number
}

export function LoadingState({ rows = 5, cols = 4 }: LoadingStateProps) {
  return (
    <div className="animate-pulse">
      {/* Skeleton header */}
      <div className="h-8 bg-slate-700/50 rounded mb-4 w-1/3" />
      {/* Skeleton rows */}
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="flex gap-4 mb-3">
          {Array.from({ length: cols }).map((__, j) => (
            <div
              key={j}
              className="h-5 bg-slate-700/40 rounded"
              style={{ flex: j === 0 ? 2 : 1 }}
            />
          ))}
        </div>
      ))}
    </div>
  )
}

export function CardSkeleton({ className = '' }: { className?: string }) {
  return (
    <div className={`animate-pulse bg-slate-800 rounded-xl p-5 border border-slate-700 ${className}`}>
      <div className="h-4 bg-slate-700/60 rounded w-1/3 mb-3" />
      <div className="h-8 bg-slate-700/40 rounded w-1/2 mb-2" />
      <div className="h-3 bg-slate-700/30 rounded w-2/3" />
    </div>
  )
}

export function ChartSkeleton({ height = 200 }: { height?: number }) {
  return (
    <div className="animate-pulse bg-slate-800 rounded-xl border border-slate-700 p-4">
      <div className="h-4 bg-slate-700/60 rounded w-1/4 mb-4" />
      <div
        className="bg-slate-700/30 rounded"
        style={{ height }}
      />
    </div>
  )
}
