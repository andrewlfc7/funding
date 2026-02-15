import { FUNDING_PERIODS_PER_DAY, DAYS_PER_YEAR } from './constants'
import type { DisplayMode, SpreadUnit, ArbOpportunity } from './types'

export function formatNumber(value: number): string {
  if (value >= 1e9) return (value / 1e9).toFixed(2) + 'B'
  if (value >= 1e6) return (value / 1e6).toFixed(2) + 'M'
  if (value >= 1e3) return (value / 1e3).toFixed(2) + 'K'
  return value.toFixed(0)
}

export function formatTimestamp(ts: string): string {
  return new Date(ts).toLocaleString()
}

export function formatRate(rate: number, displayMode: DisplayMode, unit: SpreadUnit = 'percentage'): string {
  let value: number;
  
  if (displayMode === 'annualized') {
    value = rate * FUNDING_PERIODS_PER_DAY * DAYS_PER_YEAR
  } else {
    value = rate
  }
  
  if (unit === 'percentage') {
    return (value * 100).toFixed(3) + '%'
  } else {
    return (value * 10000).toFixed(0) + ' bps'
  }
}

export function formatSpread(spreadValue: number, spreadUnit: SpreadUnit): string {
  if (spreadUnit === 'percentage') {
    return (spreadValue / 100).toFixed(3) + '%'
  }
  return spreadValue.toFixed(0) + ' bps'
}

export function formatArbOpportunity(arb: ArbOpportunity | null): string {
  if (!arb) return '–'
  
  // Return just the exchange names, the styling will be handled by CSS
  return `${arb.longExchange} → ${arb.shortExchange}`
}

export function formatVolumeForChart(value: number): string {
  if (!value || value === 0) return '$0'
  if (value >= 1e9) return '$' + (value / 1e9).toFixed(1) + 'B'
  if (value >= 1e6) return '$' + (value / 1e6).toFixed(1) + 'M'
  if (value >= 1e3) return '$' + (value / 1e3).toFixed(1) + 'K'
  return '$' + value.toFixed(0)
}

export function formatPercentile(value: number): string {
  return value.toFixed(1);
}