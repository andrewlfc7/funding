export interface ExchangeData {
  funding_rate: number
  open_interest: number
}

export interface TokenRow {
  token: string
  exchanges: Record<string, ExchangeData>
}

export interface ApiResponse {
  last_updated: string
  tokens: TokenRow[]
}

export interface ArbOpportunity {
  longExchange: string
  shortExchange: string
  longRate: number
  shortRate: number
  spread: number
  combinedOI: number
}

export type DisplayMode = 'rate' | 'annualized'
export type SpreadUnit = 'percentage' | 'bps'
export type SortDirection = 'asc' | 'desc'