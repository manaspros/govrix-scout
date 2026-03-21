import { type LucideIcon, Lock } from 'lucide-react'

interface EnterpriseFeatureCardProps {
  icon?: LucideIcon
  title: string
  description: string
  learnMoreUrl?: string
}

/**
 * Shown when an enterprise-only API endpoint returns 404.
 * Provides a clean message instead of fake/demo data.
 */
export function EnterpriseFeatureCard({
  icon: Icon = Lock,
  title,
  description,
  learnMoreUrl,
}: EnterpriseFeatureCardProps) {
  return (
    <div className="flex flex-col items-center justify-center py-20 text-center">
      <div className="flex items-center justify-center w-16 h-16 rounded-full bg-slate-700/50 border border-slate-600/30 mb-5">
        <Icon className="w-8 h-8 text-slate-400" />
      </div>
      <h3 className="text-lg font-semibold text-slate-200 font-display mb-2">{title}</h3>
      <p className="text-sm text-slate-400 max-w-md leading-relaxed">{description}</p>
      {learnMoreUrl && (
        <a
          href={learnMoreUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="mt-4 text-sm text-brand-400 hover:text-brand-300 transition-colors"
        >
          Learn more about Govrix Enterprise
        </a>
      )}
    </div>
  )
}

/**
 * Check whether a query error is a 404 (enterprise endpoint not available).
 */
export function isNotFoundError(error: unknown): boolean {
  if (error instanceof Error) {
    return error.message.includes('404') || error.message.includes('Not Found')
  }
  return false
}
