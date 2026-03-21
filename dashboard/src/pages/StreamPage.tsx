import { EventStream } from '@/components/EventStream'

export function StreamPage() {
  return (
    <div className="space-y-4 stagger-in">
      <div>
        <h1 className="text-lg font-semibold text-slate-100 font-display">Live Event Stream</h1>
        <p className="text-xs text-slate-500 mt-1">
          Real-time feed from the proxy — new events appear automatically via SSE.
        </p>
      </div>
      <EventStream fullPage maxHeight={700} />
    </div>
  )
}
