/**
 * TanStack Query hooks wrapping each API call.
 * Live data (events, health) refreshes every 5 seconds.
 * Stable data (agents, config) refreshes every 30 seconds.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import {
  fetchEvents,
  fetchEvent,
  fetchSessionEvents,
  fetchAgents,
  fetchAgent,
  updateAgent,
  retireAgent,
  fetchCostSummary,
  fetchCostBreakdown,
  fetchReportTypes,
  fetchReports,
  generateReport,
  fetchHealth,
  fetchConfig,
} from './client'
import type {
  EventFilters,
  AgentFilters,
  CostParams,
} from './types'

// ── Query keys ────────────────────────────────────────────────────────────────

export const qk = {
  events: (filters?: EventFilters) => ['events', filters] as const,
  event: (id: string) => ['events', id] as const,
  sessionEvents: (sessionId: string) => ['events', 'session', sessionId] as const,
  agents: (filters?: AgentFilters) => ['agents', filters] as const,
  agent: (id: string) => ['agents', id] as const,
  costSummary: (params?: CostParams) => ['costs', 'summary', params] as const,
  costBreakdown: (params?: CostParams) => ['costs', 'breakdown', params] as const,
  reportTypes: () => ['reports', 'types'] as const,
  reports: () => ['reports', 'list'] as const,
  health: () => ['health'] as const,
  config: () => ['config'] as const,
}

// ── Events hooks ──────────────────────────────────────────────────────────────

export function useEvents(filters?: EventFilters, autoRefresh = false) {
  return useQuery({
    queryKey: qk.events(filters),
    queryFn: () => fetchEvents(filters),
    refetchInterval: autoRefresh ? 5_000 : false,
    staleTime: 5_000,
  })
}

export function useEvent(id: string) {
  return useQuery({
    queryKey: qk.event(id),
    queryFn: () => fetchEvent(id),
    staleTime: 30_000,
  })
}

export function useSessionEvents(sessionId: string) {
  return useQuery({
    queryKey: qk.sessionEvents(sessionId),
    queryFn: () => fetchSessionEvents(sessionId),
    staleTime: 10_000,
  })
}

// ── Agents hooks ──────────────────────────────────────────────────────────────

export function useAgents(filters?: AgentFilters) {
  return useQuery({
    queryKey: qk.agents(filters),
    queryFn: () => fetchAgents(filters),
    refetchInterval: 30_000,
    staleTime: 15_000,
  })
}

export function useAgent(id: string) {
  return useQuery({
    queryKey: qk.agent(id),
    queryFn: () => fetchAgent(id),
    staleTime: 15_000,
  })
}

export function useUpdateAgent() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: Partial<{ name: string; description: string; labels: Record<string, unknown> }> }) =>
      updateAgent(id, data),
    onSuccess: (_, { id }) => {
      void queryClient.invalidateQueries({ queryKey: ['agents'] })
      void queryClient.invalidateQueries({ queryKey: qk.agent(id) })
    },
  })
}

export function useRetireAgent() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => retireAgent(id),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['agents'] })
    },
  })
}

// ── Cost hooks ────────────────────────────────────────────────────────────────

export function useCostSummary(params?: CostParams) {
  return useQuery({
    queryKey: qk.costSummary(params),
    queryFn: () => fetchCostSummary(params),
    refetchInterval: 30_000,
    staleTime: 15_000,
  })
}

export function useCostBreakdown(params?: CostParams) {
  return useQuery({
    queryKey: qk.costBreakdown(params),
    queryFn: () => fetchCostBreakdown(params),
    refetchInterval: 60_000,
    staleTime: 30_000,
  })
}

// ── Reports hooks ─────────────────────────────────────────────────────────────

export function useReportTypes() {
  return useQuery({
    queryKey: qk.reportTypes(),
    queryFn: fetchReportTypes,
    staleTime: 300_000, // report types rarely change
  })
}

export function useReports() {
  return useQuery({
    queryKey: qk.reports(),
    queryFn: fetchReports,
    staleTime: 10_000,
  })
}

export function useGenerateReport() {
  const queryClient = useQueryClient()
  return useMutation({
    mutationFn: (reportType: string) => generateReport(reportType),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: qk.reports() })
    },
  })
}

// ── Health & Config hooks ─────────────────────────────────────────────────────

export function useHealth() {
  return useQuery({
    queryKey: qk.health(),
    queryFn: fetchHealth,
    refetchInterval: 5_000,
    staleTime: 4_000,
    retry: 1,
  })
}

export function useConfig() {
  return useQuery({
    queryKey: qk.config(),
    queryFn: fetchConfig,
    staleTime: 60_000,
    retry: 1,
  })
}
