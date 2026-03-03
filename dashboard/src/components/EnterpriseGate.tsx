import { Lock } from 'lucide-react'

interface Props {
  feature: string
  description: string
}

export default function EnterpriseGate({ feature: _feature, description }: Props) {
  return (
    <div className="flex-1 flex items-center justify-center p-12">
      <div className="bg-white dark:bg-[#11111b] border border-slate-200 dark:border-[#272737] rounded-2xl p-10 text-center max-w-md shadow-lg">
        <div className="w-14 h-14 rounded-full bg-indigo-50 dark:bg-indigo-950/50 flex items-center justify-center mx-auto mb-5">
          <Lock className="w-7 h-7 text-indigo-400" />
        </div>
        <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100 mb-2">Platform Feature</h2>
        <p className="text-slate-500 text-sm leading-relaxed mb-6">{description}</p>
        <a
          href="https://govrix.io/platform"
          target="_blank"
          rel="noopener noreferrer"
          className="btn-primary inline-block w-full text-center py-3 rounded-xl"
        >
          Learn about Platform
        </a>
        <p className="text-xs text-slate-400 mt-3">Free for open-source projects</p>
      </div>
    </div>
  )
}
