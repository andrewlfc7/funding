import type { ExchangeData, ArbOpportunity } from './types'
import { SPREAD_THRESHOLDS, MIN_SPREAD_THRESHOLD_BPS } from './constants'

export function getArbClass(spreadBps: number | null): string {
  if (spreadBps === null) return ''
  
  // Use raw basis points for classification
  if (spreadBps >= 50) return 'arb-extreme'      // 50+ bps (0.5%+) spread
  if (spreadBps >= 25) return 'arb-very-high'    // 25+ bps (0.25%+) spread
  if (spreadBps >= SPREAD_THRESHOLDS.HIGH) return 'arb-high'
  if (spreadBps >= SPREAD_THRESHOLDS.MEDIUM) return 'arb-medium'
  return 'arb-low'
}

export function getArbOpportunityClass(arb: ArbOpportunity | null): string {
  if (!arb) return ''
  
  // Check if both rates have same sign
  const bothPositive = arb.longRate > 0 && arb.shortRate > 0
  const bothNegative = arb.longRate < 0 && arb.shortRate < 0
  
  // Flag extreme spreads even when same sign
  if ((bothPositive || bothNegative) && arb.spread >= 25) {  // 25+ bps spread
    return 'arb-combo-extreme-capture'
  }
  
  // Regular classification
  if (arb.spread >= SPREAD_THRESHOLDS.EXCELLENT) return 'arb-combo-excellent'
  if (arb.spread >= SPREAD_THRESHOLDS.GOOD) return 'arb-combo-good'
  if (arb.spread >= SPREAD_THRESHOLDS.DECENT) return 'arb-combo-decent'
  return 'arb-combo-minimal'
}

export function getRateClass(
  rate: number, 
  exchanges: Record<string, ExchangeData>, 
  currentExchange: string
): string {
  const rates = Object.values(exchanges).map(ex => ex.funding_rate).sort((a, b) => b - a)
  
  if (rates.length < 2) return 'rate-neutral'
  
  const maxRate = rates[0]
  const minRate = rates[rates.length - 1]
  const spreadBps = (maxRate - minRate) * 10000
  
  // Check for extreme spreads with same-sign rates
  const allPositive = minRate > 0
  const allNegative = maxRate < 0
  
  // Flag extreme opportunities when spread is large (25+ bps)
  if ((allPositive || allNegative) && spreadBps >= 25) {
    if (rate === maxRate) return 'rate-extreme-short'
    if (rate === minRate) return 'rate-extreme-long'
  }
  
  // Regular classification
  if (spreadBps < MIN_SPREAD_THRESHOLD_BPS) return 'rate-neutral'
  
  if (rate === maxRate && spreadBps >= SPREAD_THRESHOLDS.MEDIUM) return 'rate-short-candidate'
  if (rate === minRate && spreadBps >= SPREAD_THRESHOLDS.MEDIUM) return 'rate-long-candidate'
  
  if (rate === maxRate && spreadBps >= MIN_SPREAD_THRESHOLD_BPS) return 'rate-short-opportunity'
  if (rate === minRate && spreadBps >= MIN_SPREAD_THRESHOLD_BPS) return 'rate-long-opportunity'
  
  if (rate > 0) return 'rate-positive'
  if (rate < 0) return 'rate-negative'
  
  return 'rate-neutral'
}