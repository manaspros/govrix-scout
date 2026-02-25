import { clsx } from 'clsx'

export type TimeRange = '1h' | '6h' | '24h' | '7d' | '30d'

interface TimeRangePickerProps {
  value: TimeRange
  onChange: (value: TimeRange) => void
  className?: string
}

const RANGES: { label: string; value: TimeRange }[] = [
  { label: '1h', value: '1h' },
  { label: '6h', value: '6h' },
  { label: '24h', value: '24h' },
  { label: '7d', value: '7d' },
  { label: '30d', value: '30d' },
]

export function timeRangeToDays(range: TimeRange): number {
  switch (range) {
    case '1h':  return 1
    case '6h':  return 1
    case '24h': return 1
    case '7d':  return 7
    case '30d': return 30
  }
}

export function timeRangeToSince(range: TimeRange): string {
  const now = new Date()
  switch (range) {
    case '1h':  return new Date(now.getTime() - 60 * 60 * 1000).toISOString()
    case '6h':  return new Date(now.getTime() - 6 * 60 * 60 * 1000).toISOString()
    case '24h': return new Date(now.getTime() - 24 * 60 * 60 * 1000).toISOString()
    case '7d':  return new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000).toISOString()
    case '30d': return new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000).toISOString()
  }
}

export function TimeRangePicker({ value, onChange, className }: TimeRangePickerProps) {
  return (
    <div className={clsx('flex items-center rounded-lg overflow-hidden border border-slate-600', className)}>
      {RANGES.map(r => (
        <button
          key={r.value}
          onClick={() => onChange(r.value)}
          className={clsx(
            'px-3 py-1.5 text-xs font-medium transition-colors',
            value === r.value
              ? 'bg-brand-600 text-white'
              : 'bg-slate-800 text-slate-400 hover:bg-slate-700 hover:text-slate-200',
          )}
        >
          {r.label}
        </button>
      ))}
    </div>
  )
}
