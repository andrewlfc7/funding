import { FUNDING_PERIODS_PER_DAY, DAYS_PER_YEAR } from './constants'
import type { DisplayMode, SpreadUnit } from './types'

export function formatNumber(value: number): string {
  if (value >= 1e9) return (value / 1e9).toFixed(2) + 'B'
  if (value >= 1e6) return (value / 1e6).toFixed(2) + 'M'
  if (value >= 1e3) return (value / 1e3).toFixed(2) + 'K'
  return value.toFixed(0)
}

export function formatTimestamp(ts: string): string {
  return new Date(ts).toLocaleString()
}

export function formatRate(rate: number, displayMode: DisplayMode): string {
  if (displayMode === 'annualized') {
    // Convert 8-hour rate to annualized rate
    const annualizedRate = rate * FUNDING_PERIODS_PER_DAY * DAYS_PER_YEAR * 100
    return annualizedRate.toFixed(2) + '%'
  }
  // Regular 8-hour rate
  return (rate * 100).toFixed(3) + '%'
}

export function formatSpread(spreadValue: number, spreadUnit: SpreadUnit): string {
  if (spreadUnit === 'percentage') {
    return (spreadValue / 100).toFixed(3) + '%'
  }
  return spreadValue.toFixed(0) + ' bps'
}

export function formatArbOpportunity(arb: { 
  longExchange: string; 
  shortExchange: string;
  longRate: number;
  shortRate: number;
  spread: number;
} | null): string {
  if (!arb) return '–'
  
  const shortName = (ex: string) => ex.substring(0, 3).toUpperCase()
  
  // Check if it's a same-sign opportunity
  const bothPositive = arb.longRate > 0 && arb.shortRate > 0
  const bothNegative = arb.longRate < 0 && arb.shortRate < 0
  
  // Extreme capture opportunity (same sign, large spread)
  if ((bothPositive || bothNegative) && arb.spread >= 50) {  // 50+ bps
    return `🔥 L:${shortName(arb.longExchange)} S:${shortName(arb.shortExchange)}`
  }
  
  if ((bothPositive || bothNegative) && arb.spread >= 25) {  // 25+ bps
    return `🎯 L:${shortName(arb.longExchange)} S:${shortName(arb.shortExchange)}`
  }
  
  // Regular high spread (opposite signs)
  if (arb.spread >= 25) {
    return `⚡ L:${shortName(arb.longExchange)} S:${shortName(arb.shortExchange)}`
  }
  
  // Standard format
  return `L:${shortName(arb.longExchange)} S:${shortName(arb.shortExchange)}`
}