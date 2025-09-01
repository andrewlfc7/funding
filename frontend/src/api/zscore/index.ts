// src/api/zscore/index.ts
import axios from 'axios'
import {
  API_BASE_URL,
  ZSCORE_OVERVIEW_ENDPOINT,
  ZSCORE_VOLATILITY_ENDPOINT,
  ZSCORE_CROSS_ASSET_ENDPOINT,
  ZSCORE_VOL_LIQ_ENDPOINT,            // <-- add
  joinUrl,
} from '@/utils/constants'
import { handleApiError } from '@/api/error'

// -------------------- Types (mirror backend) --------------------

export type MarketType = 'spot' | 'perps'
export type Timeframe = '1h' | '4h' | '1d'

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
  statistics?: {
    meanVolatility: number
    currentVolatility: number
    volatilityPercentile: number
  }
}

// Cross-Asset Matrix
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
  // each point typically corresponds to the latest bar per coin
  volZScoreVsReturns: Array<{
    volZScore: number
    dailyRange: number   // fraction (e.g., 0.012 = 1.2%)
    volume: number       // USD notional (bar or EWMA)
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

// -------------------- Helpers (exported) --------------------

export function groupDataBySymbol<T extends { symbol: string }>(data: T[]): Map<string, T[]> {
  const grouped = new Map<string, T[]>()
  for (const point of data) {
    const list = grouped.get(point.symbol)
    if (list) list.push(point)
    else grouped.set(point.symbol, [point])
  }
  return grouped
}

export function getLatestBySymbol<T extends { symbol: string; timestamp: number }>(data: T[]): Map<string, T> {
  const latest = new Map<string, T>()
  for (const point of data) {
    const prev = latest.get(point.symbol)
    if (!prev || point.timestamp > prev.timestamp) {
      latest.set(point.symbol, point)
    }
  }
  return latest
}

// -------------------- Transforms / adapters --------------------

export function transformZScoreToTimeSeries(data: ZScoreDataPoint[]) {
  return data.map((d) => ({
    timestamp: d.timestamp * 1000, // seconds -> ms
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
    timestamp: d.timestamp * 1000, // seconds -> ms
    volatility: d.volatility * 100, // %
    volatilityZScore: d.volatilityZScore,
    returns: d.returns * 100, // %
    range: d.range * 100, // %
    symbol: d.symbol,
  }))
}
export const volrowToSeries = transformVolatilityToTimeSeries

// -------------------- API calls --------------------

// Z-Score Overview (single coin or universe via topN)
export async function fetchZScoreOverview(params: {
  exchange: string
  marketType?: MarketType
  timeframe: Timeframe
  period: string
  baseCoin?: string
  compareCoin?: string
  topN?: number
}): Promise<ZScoreOverviewResponse> {
  try {
    const url = joinUrl(API_BASE_URL, ZSCORE_OVERVIEW_ENDPOINT)
    const { data } = await axios.get<ZScoreOverviewResponse>(url, {
      params: { marketType: 'spot', ...params },
    })
    return data
  } catch (err) {
    throw handleApiError(err)
  }
}

// Volatility Analysis (single coin or universe via topN)
export async function fetchVolatilityAnalysis(params: {
  exchange: string
  marketType?: MarketType
  timeframe: Timeframe
  period: string
  coin?: string
  topN?: number
}): Promise<VolatilityAnalysisResponse> {
  try {
    const url = joinUrl(API_BASE_URL, ZSCORE_VOLATILITY_ENDPOINT)
    const { data } = await axios.get<VolatilityAnalysisResponse>(url, {
      params: { marketType: 'spot', ...params },
    })
    return data
  } catch (err) {
    throw handleApiError(err)
  }
}

// Cross-Asset Matrix (BTC as index by default; can pass any indexCoin)
export async function fetchCrossAssetMatrix(params: {
  exchange: string
  marketType?: MarketType
  timeframe: Timeframe
  period: string
  window: number
  indexCoin: string
  topN?: number
  compareCoins?: string[]
}): Promise<CrossAssetMatrixResponse> {
  try {
    const url = joinUrl(API_BASE_URL, ZSCORE_CROSS_ASSET_ENDPOINT)
    const { data } = await axios.get<CrossAssetMatrixResponse>(url, {
      params: { marketType: 'spot', ...params },
      paramsSerializer: (p) => {
        const usp = new URLSearchParams()
        Object.entries(p).forEach(([k, v]) => {
          if (Array.isArray(v)) v.forEach((item) => usp.append(k, String(item)))
          else if (v !== undefined && v !== null) usp.append(k, String(v))
        })
        return usp.toString()
      },
    })
    return data
  } catch (err) {
    throw handleApiError(err)
  }
}

// Volatility & Liquidity (single coin or universe via topN)
export async function fetchVolatilityLiquidity(params: {
  exchange: string
  marketType?: MarketType
  timeframe: Timeframe
  period: string
  coin?: string         // if omitted + topN provided => universe mode
  topN?: number
}): Promise<VolatilityLiquidityResponse> {
  try {
    const url = joinUrl(API_BASE_URL, ZSCORE_VOL_LIQ_ENDPOINT)
    const { data } = await axios.get<VolatilityLiquidityResponse>(url, {
      params: { marketType: 'spot', ...params },
    })
    return data
  } catch (err) {
    throw handleApiError(err)
  }
}
