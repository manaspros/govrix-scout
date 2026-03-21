import { useState } from 'react'
import { Globe, ChevronDown, ChevronUp, Shield, FileText, Clock, Download, AlertTriangle, CheckCircle2, Info } from 'lucide-react'

// ── EU AI Act Article Translations ──────────────────────────────────────────

interface ArticleTranslation {
  articleRef: string
  title: string
  plain: string
  whatItMeans: string
  risk: 'high' | 'medium' | 'low'
  govrixControl: string
  controlStatus: 'active' | 'partial' | 'planned'
  technicalDetail: string
}

const TRANSLATIONS: ArticleTranslation[] = [
  {
    articleRef: 'Article 13 — Transparency',
    title: 'AI Must Be Transparent',
    plain: 'Every time an AI agent acts on behalf of your organization, there must be a clear, auditable record of what it did, why, and what data it used.',
    whatItMeans: 'Your AI agents cannot operate as "black boxes." Regulators can request proof that you know what your agents are doing at any time.',
    risk: 'high',
    govrixControl: 'Audit Trail + Session Recording',
    controlStatus: 'active',
    technicalDetail: 'Every agent request is logged to PostgreSQL with SHA-256 integrity hashes. Session recordings provide full forensic replay capability.',
  },
  {
    articleRef: 'Article 14 — Human Oversight',
    title: 'Humans Must Stay in Control',
    plain: 'A human operator must be able to stop any AI agent instantly. There must be clear escalation paths and the ability to override AI decisions.',
    whatItMeans: 'You need a "big red button" — the ability to shut down any agent immediately if it behaves unexpectedly. Automated circuit breakers must supplement manual controls.',
    risk: 'high',
    govrixControl: 'Kill Switch + Circuit Breakers',
    controlStatus: 'active',
    technicalDetail: 'Manual kill/revive per agent, tenant-wide emergency stop, 4 automatic circuit breakers (token burn, consecutive errors, PII flood, loop detection).',
  },
  {
    articleRef: 'Article 10 — Data Governance',
    title: 'Protect Personal Data',
    plain: 'Any personal information (names, emails, SSNs, etc.) flowing through AI agents must be detected, masked, and protected. You must prove no personal data leaks to unauthorized systems.',
    whatItMeans: 'If an employee accidentally sends customer SSNs through an AI agent, your system must catch it before it reaches the AI provider. "We didn\'t know" is not a defense.',
    risk: 'high',
    govrixControl: 'PII Detection & Masking',
    controlStatus: 'active',
    technicalDetail: 'Real-time regex detection for 5 PII types (email, phone, SSN, credit card, API keys). Bi-directional masking with secure vault for round-trip capability.',
  },
  {
    articleRef: 'Article 9 — Risk Management',
    title: 'Manage AI Risks Proactively',
    plain: 'Your organization must have a system to evaluate, score, and mitigate risks from AI agents — including runaway costs, policy violations, and security incidents.',
    whatItMeans: 'Regulators want to see that you don\'t just react to AI incidents — you actively prevent them with budget controls, policy rules, and monitoring.',
    risk: 'medium',
    govrixControl: 'Policy Engine + Budget Controls',
    controlStatus: 'active',
    technicalDetail: 'YAML-based policy rules with hot-reload, progressive budget enforcement (warn 80% → throttle 90% → block 100%), per-agent and global limits.',
  },
  {
    articleRef: 'Article 12 — Record-Keeping',
    title: 'Keep Detailed Records',
    plain: 'All AI system activity must be recorded and retained for at least the period specified by regulators. Records must be tamper-proof and available on request.',
    whatItMeans: 'When an auditor asks "show me everything Agent X did last Tuesday," you need to produce that report within hours, not weeks.',
    risk: 'medium',
    govrixControl: 'Reports + Compliance Frameworks',
    controlStatus: 'active',
    technicalDetail: '4 report types (usage summary, cost breakdown, agent inventory, activity log). Compliance mapping for SOC 2, EU AI Act, HIPAA, NIST 800-53.',
  },
  {
    articleRef: 'Article 15 — Accuracy & Robustness',
    title: 'AI Must Be Reliable',
    plain: 'AI agents must perform consistently and reliably. When they fail, failures must be detected quickly and contained before they cascade.',
    whatItMeans: 'An agent stuck in a loop burning tokens at 3 AM shouldn\'t run up a $50,000 bill before someone notices. Automatic safeguards must kick in.',
    risk: 'medium',
    govrixControl: 'Circuit Breakers + Smart Routing',
    controlStatus: 'active',
    technicalDetail: 'Automatic loop detection via SHA-256 body hashing, consecutive error tracking, token burn rate monitoring. Failover routing to healthy providers.',
  },
  {
    articleRef: 'Article 17 — Quality Management',
    title: 'Prove Your Governance Quality',
    plain: 'Organizations must maintain a quality management system that documents AI governance processes, controls, and continuous improvement.',
    whatItMeans: 'Insurance carriers and regulators want a "governance score" — evidence that you\'re actively managing AI risk, not just hoping for the best.',
    risk: 'low',
    govrixControl: 'Insurance Evidence Package',
    controlStatus: 'active',
    technicalDetail: 'Automated evidence generation mapping 7 governance controls to insurance underwriting requirements (CIS, NIST, SOC 2, ISO 27001).',
  },
]

// ── Timeline Data ───────────────────────────────────────────────────────────

interface TimelineEvent {
  date: string
  label: string
  description: string
  isNext: boolean
}

const TIMELINE: TimelineEvent[] = [
  { date: 'Aug 2024', label: 'AI Act Published', description: 'EU AI Act entered into force in the Official Journal', isNext: false },
  { date: 'Feb 2025', label: 'Prohibited AI Banned', description: 'Unacceptable-risk AI systems banned (social scoring, emotion detection)', isNext: false },
  { date: 'Aug 2025', label: 'GPAI Rules Apply', description: 'General-Purpose AI model obligations take effect', isNext: false },
  { date: 'Aug 2026', label: 'Full Enforcement', description: 'All high-risk AI system obligations enforced — fines up to €35M or 7% of global turnover', isNext: true },
  { date: 'Aug 2027', label: 'Embedded AI', description: 'Rules apply to AI embedded in regulated products (medical, automotive)', isNext: false },
]

// ── Components ──────────────────────────────────────────────────────────────

function RiskBadge({ risk }: { risk: 'high' | 'medium' | 'low' }) {
  const colors = {
    high: 'bg-rose-500/10 text-rose-400 border-rose-500/20',
    medium: 'bg-amber-500/10 text-amber-400 border-amber-500/20',
    low: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20',
  }
  return (
    <span className={`px-2 py-0.5 rounded-full text-xs font-medium border ${colors[risk]}`}>
      {risk.toUpperCase()} PRIORITY
    </span>
  )
}

function StatusBadge({ status }: { status: 'active' | 'partial' | 'planned' }) {
  const config = {
    active: { color: 'bg-emerald-500/10 text-emerald-400 border-emerald-500/20', icon: CheckCircle2, label: 'Active' },
    partial: { color: 'bg-amber-500/10 text-amber-400 border-amber-500/20', icon: AlertTriangle, label: 'Partial' },
    planned: { color: 'bg-slate-500/10 text-slate-400 border-slate-500/20', icon: Clock, label: 'Planned' },
  }
  const { color, icon: Icon, label } = config[status]
  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium border ${color}`}>
      <Icon className="w-3 h-3" />
      {label}
    </span>
  )
}

function ArticleCard({ article }: { article: ArticleTranslation }) {
  const [expanded, setExpanded] = useState(false)

  return (
    <div className="bg-[var(--govrix-surface)] border border-[var(--govrix-border)] rounded-xl overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full px-5 py-4 flex items-start gap-4 text-left hover:bg-white/[0.02] transition-colors"
      >
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1 flex-wrap">
            <span className="text-xs font-mono text-slate-500">{article.articleRef}</span>
            <RiskBadge risk={article.risk} />
            <StatusBadge status={article.controlStatus} />
          </div>
          <h3 className="text-sm font-semibold text-slate-200 mb-1">{article.title}</h3>
          <p className="text-xs text-slate-400 leading-relaxed">{article.plain}</p>
        </div>
        {expanded ? (
          <ChevronUp className="w-4 h-4 text-slate-500 mt-1 shrink-0" />
        ) : (
          <ChevronDown className="w-4 h-4 text-slate-500 mt-1 shrink-0" />
        )}
      </button>

      {expanded && (
        <div className="px-5 pb-4 space-y-3 border-t border-[var(--govrix-border)] pt-3">
          <div className="bg-amber-500/5 border border-amber-500/10 rounded-lg p-3">
            <div className="flex items-center gap-1.5 mb-1">
              <AlertTriangle className="w-3.5 h-3.5 text-amber-400" />
              <span className="text-xs font-semibold text-amber-400">What This Means for You</span>
            </div>
            <p className="text-xs text-slate-300 leading-relaxed">{article.whatItMeans}</p>
          </div>

          <div className="bg-brand-500/5 border border-brand-500/10 rounded-lg p-3">
            <div className="flex items-center gap-1.5 mb-1">
              <Shield className="w-3.5 h-3.5 text-brand-400" />
              <span className="text-xs font-semibold text-brand-400">Govrix Control: {article.govrixControl}</span>
            </div>
            <p className="text-xs text-slate-400 leading-relaxed">{article.technicalDetail}</p>
          </div>
        </div>
      )}
    </div>
  )
}

// ── Main Page ───────────────────────────────────────────────────────────────

export function EuAiActPage() {
  const activeControls = TRANSLATIONS.filter(t => t.controlStatus === 'active').length
  const totalControls = TRANSLATIONS.length
  const highPriority = TRANSLATIONS.filter(t => t.risk === 'high').length

  const handleExportSummary = () => {
    const content = TRANSLATIONS.map(t =>
      `${t.articleRef}\n${t.title}\n${'-'.repeat(40)}\nPlain Language: ${t.plain}\nImpact: ${t.whatItMeans}\nGovrix Control: ${t.govrixControl} (${t.controlStatus})\nPriority: ${t.risk}\n`
    ).join('\n')

    const blob = new Blob([`EU AI Act Compliance Summary — Govrix\nGenerated: ${new Date().toISOString()}\n${'='.repeat(60)}\n\n${content}`], { type: 'text/plain' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `eu-ai-act-summary-${new Date().toISOString().slice(0, 10)}.txt`
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-6">
      {/* Hero */}
      <div className="bg-gradient-to-br from-blue-600/10 via-brand-500/5 to-purple-600/10 border border-blue-500/20 rounded-xl p-6">
        <div className="flex items-start gap-4">
          <div className="p-2.5 bg-blue-500/10 rounded-lg border border-blue-500/20">
            <Globe className="w-6 h-6 text-blue-400" />
          </div>
          <div className="flex-1">
            <h2 className="text-lg font-display font-bold text-slate-100 mb-1">EU AI Act Compliance</h2>
            <p className="text-sm text-slate-400 leading-relaxed max-w-2xl">
              Plain-language translations of EU AI Act requirements mapped to your Govrix governance controls.
              Designed for Data Protection Officers, compliance teams, and board-level reporting.
            </p>
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-4">
        {[
          { label: 'Articles Covered', value: totalControls, icon: FileText, color: 'text-brand-400' },
          { label: 'Controls Active', value: `${activeControls}/${totalControls}`, icon: CheckCircle2, color: 'text-emerald-400' },
          { label: 'High Priority', value: highPriority, icon: AlertTriangle, color: 'text-rose-400' },
          { label: 'Days to Enforcement', value: Math.max(0, Math.floor((new Date('2026-08-02').getTime() - Date.now()) / 86400000)), icon: Clock, color: 'text-amber-400' },
        ].map(({ label, value, icon: Icon, color }) => (
          <div key={label} className="bg-[var(--govrix-surface)] border border-[var(--govrix-border)] rounded-xl p-4">
            <div className="flex items-center gap-2 mb-2">
              <Icon className={`w-4 h-4 ${color}`} />
              <span className="text-xs text-slate-500">{label}</span>
            </div>
            <div className="text-xl font-bold text-slate-200">{value}</div>
          </div>
        ))}
      </div>

      {/* Export Buttons */}
      <div className="flex gap-3">
        <button
          onClick={handleExportSummary}
          className="flex items-center gap-2 px-4 py-2 bg-brand-600/20 hover:bg-brand-600/30 border border-brand-500/30 rounded-lg text-sm text-brand-300 font-medium transition-colors"
        >
          <Download className="w-4 h-4" />
          Export Board Summary
        </button>
        <button
          onClick={handleExportSummary}
          className="flex items-center gap-2 px-4 py-2 bg-slate-700/30 hover:bg-slate-700/50 border border-slate-600/30 rounded-lg text-sm text-slate-300 font-medium transition-colors"
        >
          <FileText className="w-4 h-4" />
          Export Technical Report
        </button>
      </div>

      {/* Timeline */}
      <div className="bg-[var(--govrix-surface)] border border-[var(--govrix-border)] rounded-xl p-5">
        <h3 className="text-sm font-semibold text-slate-200 mb-4 flex items-center gap-2">
          <Clock className="w-4 h-4 text-slate-400" />
          EU AI Act Timeline
        </h3>
        <div className="flex items-center gap-0">
          {TIMELINE.map((event, i) => (
            <div key={event.date} className="flex-1 relative">
              <div className="flex flex-col items-center text-center">
                <div className={`w-3 h-3 rounded-full border-2 z-10 ${
                  event.isNext
                    ? 'bg-amber-400 border-amber-400 shadow-lg shadow-amber-400/30'
                    : 'bg-slate-700 border-slate-600'
                }`} />
                {i < TIMELINE.length - 1 && (
                  <div className="absolute top-1.5 left-1/2 w-full h-px bg-slate-700" />
                )}
                <span className={`text-[0.625rem] font-mono mt-2 ${event.isNext ? 'text-amber-400 font-bold' : 'text-slate-500'}`}>
                  {event.date}
                </span>
                <span className={`text-[0.6875rem] font-medium mt-0.5 ${event.isNext ? 'text-amber-300' : 'text-slate-400'}`}>
                  {event.label}
                </span>
                <span className="text-[0.5625rem] text-slate-600 mt-0.5 px-2 leading-tight">
                  {event.description}
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Article Cards */}
      <div className="space-y-3">
        <h3 className="text-sm font-semibold text-slate-200 flex items-center gap-2">
          <Info className="w-4 h-4 text-slate-400" />
          Article-by-Article Translation
        </h3>
        {TRANSLATIONS.map((article) => (
          <ArticleCard key={article.articleRef} article={article} />
        ))}
      </div>

      {/* Footer */}
      <div className="text-center py-4">
        <p className="text-xs text-slate-600">
          This translation is provided for informational purposes. Consult qualified legal counsel for formal compliance assessment.
        </p>
      </div>
    </div>
  )
}
