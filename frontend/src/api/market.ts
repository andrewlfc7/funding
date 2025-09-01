// frontend/src/api/trend.ts
import axios from 'axios'
import {
  API_BASE_URL,
  KLINES_ENDPOINT,        // '/api/klines/daily'
  RETURNS_ENDPOINT,       // '/api/signals/returns'
  VOLATILITY_ENDPOINT,    // '/api/signals/volatility'
} from '@/utils/constants'
import { handleApiError } from './error'

// ---- Types ----
export type MarketType = 'spot' | 'perps'

export interface KlineDTO {
  ts: number
  open: number
  high: number
  low: number
  close: number
  volume: number
}

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

// Ensure we only send uppercase base symbols (no auto-quote/pairing)
function toBase(symbol: string): string {
  return symbol.toUpperCase().trim()
}

// ---- API calls ----
export async function fetchDailyKlines(params: BaseParams): Promise<KlineDTO[]> {
  try {
    const url = `${API_BASE_URL}${KLINES_ENDPOINT}`
    const p = { ...params, symbol: toBase(params.symbol) }
    const { data } = await axios.get<KlineDTO[]>(url, { params: p })
    return data
  } catch (e) {
    throw new Error(handleApiError(e, 'fetchDailyKlines'))
  }
}

export async function fetchReturns(params: RVParams): Promise<ReturnPoint[]> {
  try {
    const url = `${API_BASE_URL}${RETURNS_ENDPOINT}`
    const p = { ...params, symbol: toBase(params.symbol) }
    const { data } = await axios.get<ReturnPoint[]>(url, { params: p })
    return data.map(d => ({ ts: Number(d.ts), value: Number(d.value) }))
  } catch (e) {
    throw new Error(handleApiError(e, 'fetchReturns'))
  }
}

export async function fetchVolatility(params: RVParams): Promise<VolPoint[]> {
  try {
    const url = `${API_BASE_URL}${VOLATILITY_ENDPOINT}`
    const p = { ...params, symbol: toBase(params.symbol) }
    const { data } = await axios.get<VolPoint[]>(url, { params: p })
    return data.map(d => ({ ts: Number(d.ts), value: Number(d.value) }))
  } catch (e) {
    throw new Error(handleApiError(e, 'fetchVolatility'))
  }
}

// ---- Convenience grab-all ----
export async function fetchMarketInfo(params: RVParams): Promise<{
  klines: KlineDTO[]
  returns: ReturnPoint[]
  volatility: VolPoint[]
}> {
  const p = { ...params, symbol: toBase(params.symbol) }
  const [klines, returns, volatility] = await Promise.all([
    fetchDailyKlines(p),
    fetchReturns(p),
    fetchVolatility(p),
  ])
  return { klines, returns, volatility }
}
