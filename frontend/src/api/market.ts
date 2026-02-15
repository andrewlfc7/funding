// frontend/src/api/trend.ts
import axios from 'axios'
import {
  API_BASE_URL,
  TREND_SERIES_PRICE_ENDPOINT,
  TREND_SERIES_RETURNS_ENDPOINT,
  TREND_SERIES_VOL_ENDPOINT,
  joinUrl,
} from '@/utils/constants'
import { handleApiError } from './error'

// ---- Types ----
export type MarketType = 'spot' | 'perps'

export type ReturnPoint = { ts: number; value: number }
export type VolPoint    = { ts: number; value: number }

export interface BaseParams {
  exchange: string
  market_type: MarketType
  /** Base symbol, e.g. 'BTC' */
  symbol: string
  days: number
}

export interface RVParams extends BaseParams {
  vol_window?: number
}

// Ensure we only send uppercase base symbols
function toBase(symbol: string): string {
  return symbol.toUpperCase().trim()
}

// ---- Series helpers ----
type SeriesResponse = { ts: number[]; values: number[] }

function mapSeries<T extends { ts: number; value: number }>(
  data: SeriesResponse
): T[] {
  const n = Math.min(data.ts.length, data.values.length)
  const out: T[] = new Array(n)
  for (let i = 0; i < n; i++) {
    out[i] = { ts: Number(data.ts[i]), value: Number(data.values[i]) } as T
  }
  return out
}

// ---- API calls (series) ----
export async function fetchPriceSeries(params: BaseParams): Promise<ReturnPoint[]> {
  try {
    const url = joinUrl(API_BASE_URL, TREND_SERIES_PRICE_ENDPOINT)
    const p = { ...params, symbol: toBase(params.symbol) }
    const { data } = await axios.get<SeriesResponse>(url, { params: p })
    return mapSeries<ReturnPoint>(data)
  } catch (e) {
    throw new Error(handleApiError(e, 'fetchPriceSeries'))
  }
}

export async function fetchReturns(params: RVParams): Promise<ReturnPoint[]> {
  try {
    const url = joinUrl(API_BASE_URL, TREND_SERIES_RETURNS_ENDPOINT)
    const p = { ...params, symbol: toBase(params.symbol) }
    const { data } = await axios.get<SeriesResponse>(url, { params: p })
    return mapSeries<ReturnPoint>(data)
  } catch (e) {
    throw new Error(handleApiError(e, 'fetchReturns'))
  }
}

export async function fetchVolatility(params: RVParams): Promise<VolPoint[]> {
  try {
    const url = joinUrl(API_BASE_URL, TREND_SERIES_VOL_ENDPOINT)
    const p = { ...params, symbol: toBase(params.symbol), annualize: true }
    const { data } = await axios.get<SeriesResponse>(url, { params: p })
    return mapSeries<VolPoint>(data)
  } catch (e) {
    throw new Error(handleApiError(e, 'fetchVolatility'))
  }
}

// ---- Convenience grab-all ----
export async function fetchMarketInfo(params: RVParams): Promise<{
  prices: ReturnPoint[]       // close price series
  returns: ReturnPoint[]
  volatility: VolPoint[]
}> {
  const p = { ...params, symbol: toBase(params.symbol) }
  const [prices, returns, volatility] = await Promise.all([
    fetchPriceSeries(p),
    fetchReturns(p),
    fetchVolatility(p),
  ])
  return { prices, returns, volatility }
}
