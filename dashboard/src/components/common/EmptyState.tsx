export default function EmptyState({ message = 'No data yet' }: { message?: string }) {
  return (
    <div className="flex flex-col items-center justify-center py-16 text-slate-400">
      <div className="w-10 h-10 rounded-full bg-slate-100 dark:bg-slate-800 flex items-center justify-center mb-3">
        <span className="text-lg">∅</span>
      </div>
      <p className="text-sm">{message}</p>
    </div>
  )
}
