// Shared types for all z-score API calls

export type MarketType = 'spot' | 'perps'
export type Timeframe  = '1h' | '4h' | '1d'

// ---- Common rows ----
export interface ZScoreDataPoint {
  timestamp: number
  price: number
  zscore: number
  volume: number
  returns1h: number
  returns1d: number
  logReturns1h: number
  rollingVolume?: number
  rollingDollarVolume?: number
  rollingDollarVolumeZ?: number
  rollingDollarVolumeNorm?: number
  symbol: string
}
export type ZRow = ZScoreDataPoint

export interface VolatilityDataPoint {
  timestamp: number
  volatility: number
  volatilityZScore: number
  returns: number
  high: number
  low: number
  range: number
  symbol: string
  volume?: number
}
export type VolRow = VolatilityDataPoint

// ---- Responses ----
export interface ZScoreOverviewResponse {
  zscoreTimeSeries: ZScoreDataPoint[]
  statistics?: {
    mean: number
    std: number
    min: number
    max: number
    current: number
  }
}

export interface VolatilityAnalysisResponse {
  volatilityTimeSeries: VolatilityDataPoint[]
  volVsReturns: Array<{
    volZScore: number
    returns: number
    volume: number
  }>
  rangeDistribution: {
    buckets: number[]
    counts: number[]
  }
}

// Cross-Asset
export interface CrossAssetMatrixResponse {
  correlationMatrix: {
    coins: string[]
    matrix: number[][]
    timestamp?: number
  }
  betaMatrix: {
    coins: string[]
    betas: number[]
  }
  correlationHistogram: {
    buckets: number[]
    counts: number[]
    mean: number
    std: number
  }
  betaHistogram: {
    buckets: number[]
    counts: number[]
    mean: number
    std: number
  }
}

// Volatility + Liquidity
export interface VolatilityLiquidityResponse {
  volZScoreTimeSeries: Array<{
    timestamp: number
    volZScore: number
    threshold2Sigma?: number
    thresholdNeg2Sigma?: number
    symbol?: string
  }>
  volumeDistribution: {
    buckets: number[]
    counts: number[]
    currentZScore?: number
  }
  volZScoreVsReturns: Array<{
    volZScore: number
    dailyRange: number
    volume: number
    symbol?: string
  }>
  volumeSummaries: Array<{
    symbol: string
    weeklyDollarVolume: number
    avgDailyDollarVolume: number
    avgHourlyDollarVolume: number
    currentDollarVolume: number
    ratioCurrentToAvgDaily: number
    ratioEWMAToAvgDaily: number
  }>
}

// Market Regime / Momentum
export interface MarketRegimeResponse {
  heatmap?: {
    coins: string[]
    timeframes: string[]
    matrix: number[][]
  }
  transitionMatrix?: number[][]
  velocitySeries?: Array<{
    symbol: string
    series: Array<{
      timestamp: number
      velocity: number
      acceleration?: number
    }>
  }>
  crossAssetDivergence: {
    leaders: string[]
    laggards: string[]
    score: number
  }
}

// Relative Strength (raw + normalized)
export interface RelativeStrengthResponseRaw {
  base?: string
  baseCoin?: string
  momentumFactorLoadings?: Array<{ symbol: string; beta: number; r2: number }>
  pairDivergence?: Array<{
    pair: string
    timeSeries?: Array<{ timestamp: number; z1?: number; z2?: number; divergence?: number; zscore?: number; spread?: number }>
    series?: Array<{ timestamp: number; z1?: number; z2?: number; divergence?: number; zscore?: number; spread?: number }>
  }>
  persistence?: Array<{ symbol: string; rho1: number }>
  rsRankings?:
    | Array<{ rank?: number; symbol?: string; pair?: string; name?: string; z?: number; zscore?: number; score?: number; rsZScore?: number; relativeStrengthZ?: number }>
    | Record<string, { rank?: number; symbol?: string; pair?: string; name?: string; z?: number; zscore?: number; score?: number; rsZScore?: number; relativeStrengthZ?: number }>
  rsSeries?:
    | Array<{ symbol?: string; pair?: string; name?: string; series?: Array<any>; timeSeries?: Array<any> }>
    | Record<string, { symbol?: string; pair?: string; name?: string; series?: Array<any>; timeSeries?: Array<any> }>
}

export interface RSRankRow { rank: number; symbol: string; z: number | null }
export interface RSPersistenceRow { symbol: string; rho1: number }
export interface RSMomentumFactor { symbol: string; beta: number; r2: number }
export interface RSPairDivPoint { timestamp: number; z1?: number; z2?: number; divergence?: number; zscore?: number; spread?: number }
export interface RSPairDivergence { pair: string; timeSeries: RSPairDivPoint[] }
export interface RSSeriesRow { symbol: string; series: Array<any> }

export interface RelativeStrengthResponse {
  base: string
  momentumFactorLoadings: RSMomentumFactor[]
  pairDivergence: RSPairDivergence[]
  persistence: RSPersistenceRow[]
  rsRankings: RSRankRow[]
  rsSeries: RSSeriesRow[]
}

// ---- Small client-side helpers ----
export function transformZScoreToTimeSeries(data: ZScoreDataPoint[]) {
  return data.map((d) => ({
    timestamp: d.timestamp * 1000, // s -> ms
    value: d.zscore,
    price: d.price,
    volume: d.volume,
    returns1h: d.returns1h,
    returns1d: d.returns1d,
    symbol: d.symbol,
  }))
}
export const zrowToSeries = transformZScoreToTimeSeries

export function transformVolatilityToTimeSeries(data: VolatilityDataPoint[]) {
  return data.map((d) => ({
    timestamp: d.timestamp * 1000, // s -> ms
    volatility: d.volatility * 100, // %
    volatilityZScore: d.volatilityZScore,
    returns: d.returns * 100,       // %
    range: d.range * 100,           // %
    symbol: d.symbol,
  }))
}
export const volrowToSeries = transformVolatilityToTimeSeries

export function groupDataBySymbol<T extends { symbol: string }>(data: T[]): Map<string, T[]> {
  const m = new Map<string, T[]>()
  for (const p of data) (m.get(p.symbol)?.push(p)) ?? m.set(p.symbol, [p])
  return m
}
export function getLatestBySymbol<T extends { symbol: string; timestamp: number }>(data: T[]): Map<string, T> {
  const m = new Map<string, T>()
  for (const p of data) if (!m.has(p.symbol) || p.timestamp > (m.get(p.symbol)!.timestamp)) m.set(p.symbol, p)
  return m
}
