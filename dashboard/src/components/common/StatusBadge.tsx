import { clsx } from 'clsx'

type BadgeVariant =
  | 'active'
  | 'idle'
  | 'error'
  | 'blocked'
  | 'retired'
  | 'ok'
  | 'degraded'
  | 'llm'
  | 'tool'
  | 'http'
  | 'openai'
  | 'anthropic'
  | 'mcp'
  | 'a2a'
  | 'custom'
  | 'default'

interface StatusBadgeProps {
  value: string | undefined | null
  variant?: BadgeVariant
  size?: 'sm' | 'md'
}

const variantMap: Record<string, string> = {
  active:    'bg-emerald-500/20 text-emerald-400 ring-1 ring-emerald-500/40',
  idle:      'bg-slate-500/20 text-slate-400 ring-1 ring-slate-500/40',
  error:     'bg-red-500/20 text-red-400 ring-1 ring-red-500/40',
  blocked:   'bg-orange-500/20 text-orange-400 ring-1 ring-orange-500/40',
  retired:   'bg-gray-600/20 text-gray-500 ring-1 ring-gray-600/40',
  ok:        'bg-emerald-500/20 text-emerald-400 ring-1 ring-emerald-500/40',
  degraded:  'bg-yellow-500/20 text-yellow-400 ring-1 ring-yellow-500/40',
  llm:       'bg-brand-500/20 text-brand-400 ring-1 ring-brand-500/40',
  tool:      'bg-violet-500/20 text-violet-400 ring-1 ring-violet-500/40',
  http:      'bg-slate-500/20 text-slate-300 ring-1 ring-slate-500/40',
  openai:    'bg-emerald-500/20 text-emerald-400 ring-1 ring-emerald-500/40',
  anthropic: 'bg-orange-500/20 text-orange-400 ring-1 ring-orange-500/40',
  mcp:       'bg-violet-500/20 text-violet-400 ring-1 ring-violet-500/40',
  a2a:       'bg-brand-500/20 text-brand-400 ring-1 ring-brand-500/40',
  custom:    'bg-slate-500/20 text-slate-300 ring-1 ring-slate-500/40',
  default:   'bg-slate-600/20 text-slate-400 ring-1 ring-slate-600/40',
}

function resolveVariant(value: string | undefined | null, explicit?: BadgeVariant): string {
  if (explicit && explicit !== 'default') return variantMap[explicit] ?? variantMap['default']
  if (!value) return variantMap['default']
  const lower = value.toLowerCase()
  return variantMap[lower] ?? variantMap['default']
}

export function StatusBadge({ value, variant, size = 'sm' }: StatusBadgeProps) {
  const display = value ?? 'unknown'
  const classes = resolveVariant(display, variant)
  return (
    <span
      className={clsx(
        'inline-flex items-center rounded-full font-medium capitalize',
        size === 'sm' ? 'px-2 py-0.5 text-xs' : 'px-2.5 py-1 text-sm',
        classes,
      )}
    >
      {display}
    </span>
  )
}
