const COLORS: Record<string, string> = {
  active: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  retired: 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400',
  blocked: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  ok: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  degraded: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  error: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  complete: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  pending: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  failed: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
}

export default function StatusBadge({ status }: { status: string }) {
  const cls = COLORS[status] ?? 'bg-slate-100 text-slate-600'
  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded-md text-xs font-medium capitalize ${cls}`}>
      {status}
    </span>
  )
}
