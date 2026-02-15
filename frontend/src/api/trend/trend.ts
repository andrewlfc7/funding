// src/api/trend/trend.ts
import type { MarketType } from '../../utils/types'
import {
  API_BASE_URL,
  TREND_SIGNALS_XSEC_ENDPOINT,
  TREND_SERIES_PRICE_ENDPOINT,
  TREND_SERIES_RETURNS_ENDPOINT,
  TREND_SERIES_VOL_ENDPOINT,
  TREND_SERIES_VOLUME_ENDPOINT,
  joinUrl,
} from '../../utils/constants'


const API_BASE = API_BASE_URL // keep alias
const withBase = (path: string) => joinUrl(API_BASE, path)

// Request interfaces
export interface TrendSignalsRequest {
  exchange: string
  market_type: MarketType
  symbol?: string
  days?: number
  vol_window?: number
  min_decile?: number
}

// ... (other request interfaces are unchanged) ...

// Response interfaces
export interface TrendSignalPoint {
  ts: number // This remains a number as it comes from a different endpoint
  symbol: string
  trend: number
  momentum: number
  ewmac: number
  breakout: number
  composite: number
}

export interface TimeSeriesPoint {
  date: string; // Changed from ts: number
  value: number;
}

export interface VolumePoint {
  date: string; // Changed from ts: number
  volume_ewma: number;
  dollar_volume_ewma: number;
}

export interface SignalsDataRequest {
  exchange: string
  market_type: MarketType
  days?: number
  symbol?: string
}

// ... (other response interfaces are unchanged) ...

// API functions
export async function fetchTrendSignals(params: SignalsDataRequest): Promise<TrendSignalPoint[]> {
  const qs = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    ...(params.days && { days: String(params.days) }),
    ...(params.symbol && { symbol: params.symbol }),
  })
  const url = `${withBase(TREND_SIGNALS_XSEC_ENDPOINT)}?${qs}`
  const res = await fetch(url)
  if (!res.ok) throw new Error(`Failed to fetch trend signals: ${res.statusText}`)
  return res.json()
}


export async function fetchPriceSeries(params: SignalsDataRequest): Promise<TimeSeriesPoint[]> {
  const qs = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    symbol: params.symbol ?? 'BTC',
    ...(params.days && { days: String(params.days) }),
  })
  const url = `${withBase(TREND_SERIES_PRICE_ENDPOINT)}?${qs}`
  const res = await fetch(url)
  if (!res.ok) throw new Error(`Failed to fetch price series: ${res.statusText}`)
  return res.json()
}

export async function fetchReturnsSeries(params: SignalsDataRequest): Promise<TimeSeriesPoint[]> {
  const qs = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    symbol: params.symbol ?? 'BTC',
    ...(params.days && { days: String(params.days) }),
  })
  const url = `${withBase(TREND_SERIES_RETURNS_ENDPOINT)}?${qs}`
  const res = await fetch(url)
  if (!res.ok) throw new Error(`Failed to fetch returns series: ${res.statusText}`)
  return res.json()
}

export async function fetchVolSeries(params: SignalsDataRequest): Promise<TimeSeriesPoint[]> {
  const qs = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    symbol: params.symbol ?? 'BTC',
    ...(params.days && { days: String(params.days) }),
    annualize: 'true',
  })
  const url = `${withBase(TREND_SERIES_VOL_ENDPOINT)}?${qs}`
  const res = await fetch(url)
  if (!res.ok) throw new Error(`Failed to fetch volatility series: ${res.statusText}`)
  return res.json()
}

export async function fetchVolumeSeries(
  params: SignalsDataRequest & { span?: number }
): Promise<VolumePoint[]> {
  const qs = new URLSearchParams({
    exchange: params.exchange,
    market_type: params.market_type,
    symbol: params.symbol ?? 'BTC',
    ...(params.days && { days: String(params.days) }),
    ...(params.span && { span: String(params.span) }),
  })
  const url = `${withBase(TREND_SERIES_VOLUME_ENDPOINT)}?${qs}`
  const res = await fetch(url)
  if (!res.ok) throw new Error(`Failed to fetch volume series: ${res.statusText}`)
  return res.json()
}