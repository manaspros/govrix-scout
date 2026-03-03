// dashboard/src/api/hooks.ts
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import * as api from './client'
import type { EventFilters, GenerateReportRequest } from './types'

export const qk = {
  health: ['health'] as const,
  events: (f: EventFilters) => ['events', f] as const,
  event: (id: string) => ['events', id] as const,
  sessionEvents: (sid: string) => ['events', 'session', sid] as const,
  agents: ['agents'] as const,
  agent: (id: string) => ['agents', id] as const,
  agentEvents: (id: string, f: EventFilters) => ['agents', id, 'events', f] as const,
  costSummary: ['costs', 'summary'] as const,
  costBreakdown: ['costs', 'breakdown'] as const,
  reportTypes: ['reports', 'types'] as const,
  reports: ['reports'] as const,
  config: ['config'] as const,
}

export const useHealth = () =>
  useQuery({ queryKey: qk.health, queryFn: api.getHealth, refetchInterval: 10_000 })

export const useEvents = (filters: EventFilters = {}) =>
  useQuery({ queryKey: qk.events(filters), queryFn: () => api.getEvents(filters), refetchInterval: 5_000 })

export const useEvent = (id: string) =>
  useQuery({ queryKey: qk.event(id), queryFn: () => api.getEvent(id), enabled: !!id })

export const useSessionEvents = (sessionId: string) =>
  useQuery({ queryKey: qk.sessionEvents(sessionId), queryFn: () => api.getSessionEvents(sessionId), enabled: !!sessionId })

export const useAgents = () =>
  useQuery({ queryKey: qk.agents, queryFn: api.getAgents, refetchInterval: 10_000 })

export const useAgent = (id: string) =>
  useQuery({ queryKey: qk.agent(id), queryFn: () => api.getAgent(id), enabled: !!id })

export const useAgentEvents = (id: string, filters: EventFilters = {}) =>
  useQuery({ queryKey: qk.agentEvents(id, filters), queryFn: () => api.getAgentEvents(id, filters), enabled: !!id })

export const useUpdateAgent = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: Parameters<typeof api.updateAgent>[1] }) =>
      api.updateAgent(id, body),
    onSuccess: () => { qc.invalidateQueries({ queryKey: qk.agents }) },
  })
}

export const useRetireAgent = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.retireAgent(id),
    onSuccess: () => { qc.invalidateQueries({ queryKey: qk.agents }) },
  })
}

export const useCostSummary = () =>
  useQuery({ queryKey: qk.costSummary, queryFn: api.getCostSummary, refetchInterval: 30_000 })

export const useCostBreakdown = () =>
  useQuery({ queryKey: qk.costBreakdown, queryFn: api.getCostBreakdown, refetchInterval: 30_000 })

export const useReportTypes = () =>
  useQuery({ queryKey: qk.reportTypes, queryFn: api.getReportTypes, staleTime: Infinity })

export const useReports = () =>
  useQuery({ queryKey: qk.reports, queryFn: api.getReports, refetchInterval: 15_000 })

export const useGenerateReport = () => {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: GenerateReportRequest) => api.generateReport(body),
    onSuccess: () => { qc.invalidateQueries({ queryKey: qk.reports }) },
  })
}

export const useConfig = () =>
  useQuery({ queryKey: qk.config, queryFn: api.getConfig, staleTime: 60_000 })
