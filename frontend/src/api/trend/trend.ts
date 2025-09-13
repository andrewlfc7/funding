// src/api/trend/trend.ts
import type { MarketType } from '../../utils/types'

// Base API configuration
const API_BASE = import.meta.env.VITE_API_BASE || 'http://localhost:8000'

// Request interfaces
export interface TrendSignalsRequest {
  exchange: string
  market_type: MarketType
  symbol?: string
  days?: number
  vol_window?: number
  min_decile?: number
}

export interface PortfolioConstructionRequest {
  exchange: string
  market_type: MarketType
  days?: number
  max_position?: number
  max_long?: number
  max_short?: number
  target_vol?: number
}

export interface SignalPerformanceRequest {
  exchange: string
  market_type: MarketType
  symbol: string
  days?: number
  forward_window?: string // '1d', '5d', '10d', '20d'
}

// Response interfaces
export interface SignalData {
  timestamp: number
  price: number
  volume: number
  returns: number
  volatility: number
  momentum: number
  ewmac: number
  breakout: number
  composite: number
  rank_momentum: number
  rank_ewmac: number
  rank_breakout: number
  rank_composite: number
}

export interface XSecSignalsResponse {
  symbols: string[]
  series: {
    [symbol: string]: SignalData[]
  }
  correlations: {
    [key: string]: number
  }
  summary: {
    total_assets: number
    avg_momentum: number
    avg_ewmac: number
    avg_breakout: number
    avg_composite: number
  }
}

export interface PortfolioPosition {
  asset: string
  weight: number
  volatility: number
  signal: number
  expected_return: number
  contribution: number
}

export interface PortfolioMetrics {
  total_long: number
  total_short: number
  net_exposure: number
  gross_exposure: number
  expected_return: number
  expected_vol: number
  expected_sharpe: number
  max_drawdown_risk: number
}

export interface PortfolioConstructionResponse {
  positions: PortfolioPosition[]
  metrics: PortfolioMetrics
  constraints: {
    max_position: number
    max_long: number
    max_short: number
    target_vol: number
  }
  attribution: {
    by_signal: Array<{
      signal: string
      contribution: number
    }>
    by_position: Array<{
      asset: string
      contribution: number
    }>
  }
}

export interface SignalPerformanceData {
  signal_returns: Array<{
    signal: number
    forward_return: number
    timestamp: number
  }>
  performance_metrics: {
    information_coefficient: number
    hit_rate: number
    r_squared: number
  }
  trend_decomposition: {
    systematic_trend: number[]
    idiosyncratic_trend: number[]
    reversion_component: number[]
    timestamps: number[]
  }
  volatility_analysis: {
    short_term_vol: number[]
    long_term_vol: number[]
    vol_forecast: number[]
    vol_actual: number[]
    timestamps: number[]
  }
}

export interface TrendDirectionData {
  trend_state: {
    current_trend: string
    strength: number
    reading: number
    duration: number
    regime: string
  }
  long_short: {
    ratio: number
    ratio_history: number[]
    long_strength: number
    short_strength: number
  }
  exposure: {
    net_exposure_history: number[]
    sector_bias: Array<{
      name: string
      bias: number
    }>
  }
  persistence: {
    duration_distribution: Array<{
      days: number
      frequency: number
    }>
    reversal_probability: number
    sustainability: string
  }
  regime_timing: {
    probabilities: Array<{
      regime: string
      probability: number
      color: string
    }>
    performance: Array<{
      regime: string
      pnl: number
    }>
  }
  concentration: {
    top_long: Array<{
      asset: string
      weight: number
    }>
    top_short: Array<{
      asset: string
      weight: number
    }>
    risk_metrics: {
      concentration_risk: string
      diversification_score: number
      single_position_risk: number
    }
  }
}

// API functions
export async function fetchXSecSignals(params: TrendSignalsRequest): Promise<XSecSignalsResponse> {
  const queryParams = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    ...(params.symbol && { symbol: params.symbol }),
    ...(params.days && { days: params.days.toString() }),
    ...(params.vol_window && { vol_window: params.vol_window.toString() }),
    ...(params.min_decile && { min_decile: params.min_decile.toString() })
  })

  const response = await fetch(`${API_BASE}/trend/signals/cross-sectional?${queryParams}`)
  
  if (!response.ok) {
    throw new Error(`Failed to fetch cross-sectional signals: ${response.statusText}`)
  }
  
  return response.json()
}

export async function fetchPortfolioConstruction(params: PortfolioConstructionRequest): Promise<PortfolioConstructionResponse> {
  const queryParams = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    ...(params.days && { days: params.days.toString() }),
    ...(params.max_position && { max_position: params.max_position.toString() }),
    ...(params.max_long && { max_long: params.max_long.toString() }),
    ...(params.max_short && { max_short: params.max_short.toString() }),
    ...(params.target_vol && { target_vol: params.target_vol.toString() })
  })

  const response = await fetch(`${API_BASE}/trend/portfolio/construction?${queryParams}`)
  
  if (!response.ok) {
    throw new Error(`Failed to fetch portfolio construction: ${response.statusText}`)
  }
  
  return response.json()
}

export async function fetchSignalPerformance(params: SignalPerformanceRequest): Promise<SignalPerformanceData> {
  const queryParams = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    symbol: params.symbol,
    ...(params.days && { days: params.days.toString() }),
    ...(params.forward_window && { forward_window: params.forward_window })
  })

  const response = await fetch(`${API_BASE}/trend/signals/performance?${queryParams}`)
  
  if (!response.ok) {
    throw new Error(`Failed to fetch signal performance: ${response.statusText}`)
  }
  
  return response.json()
}

export async function fetchTrendDirection(params: TrendSignalsRequest): Promise<TrendDirectionData> {
  const queryParams = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    ...(params.days && { days: params.days.toString() })
  })

  const response = await fetch(`${API_BASE}/trend/direction/analysis?${queryParams}`)
  
  if (!response.ok) {
    throw new Error(`Failed to fetch trend direction: ${response.statusText}`)
  }
  
  return response.json()
}

// Market data functions (moved from volatility.ts for completeness)
export interface KlineData {
  timestamp: number
  open: number
  high: number
  low: number
  close: number
  volume: number
}

export interface ReturnData {
  ts: number
  value: number
}

export interface VolatilityData {
  ts: number
  value: number
}

export interface MarketDataRequest {
  exchange: string
  market_type: MarketType
  symbol: string
  days: number
  vol_window?: number
}

export async function fetchKlines(params: MarketDataRequest): Promise<KlineData[]> {
  const queryParams = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    symbol: params.symbol,
    days: params.days.toString()
  })

  const response = await fetch(`${API_BASE}/market/klines?${queryParams}`)
  
  if (!response.ok) {
    throw new Error(`Failed to fetch klines: ${response.statusText}`)
  }
  
  return response.json()
}

export async function fetchReturns(params: MarketDataRequest): Promise<ReturnData[]> {
  const queryParams = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    symbol: params.symbol,
    days: params.days.toString()
  })

  const response = await fetch(`${API_BASE}/market/returns?${queryParams}`)
  
  if (!response.ok) {
    throw new Error(`Failed to fetch returns: ${response.statusText}`)
  }
  
  return response.json()
}

export async function fetchVolatility(params: MarketDataRequest): Promise<VolatilityData[]> {
  const queryParams = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    symbol: params.symbol,
    days: params.days.toString(),
    ...(params.vol_window && { vol_window: params.vol_window.toString() })
  })

  const response = await fetch(`${API_BASE}/market/volatility?${queryParams}`)
  
  if (!response.ok) {
    throw new Error(`Failed to fetch volatility: ${response.statusText}`)
  }
  
  return response.json()
}

// Portfolio optimization functions
export interface OptimizationParams {
  signals: Record<string, number[]>
  returns: number[]
  volatilities: number[]
  correlations: number[][]
  constraints: {
    max_position: number
    max_long: number
    max_short: number
    target_vol: number
  }
}

export async function optimizePortfolio(params: OptimizationParams): Promise<PortfolioConstructionResponse> {
  const response = await fetch(`${API_BASE}/trend/portfolio/optimize`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(params)
  })
  
  if (!response.ok) {
    throw new Error(`Failed to optimize portfolio: ${response.statusText}`)
  }
  
  return response.json()
}