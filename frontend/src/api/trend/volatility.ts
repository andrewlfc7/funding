import axios from 'axios'
import {
  API_BASE_URL,
  KLINES_ENDPOINT,
  RETURNS_ENDPOINT,
  VOLATILITY_ENDPOINT,
} from '@/utils/constants'
import { handleApiError } from '@/api/error'

export type MarketType = 'spot' | 'perps'
export type ReturnPoint = { ts: number; value: number }
export type VolPoint    = { ts: number; value: number }



export async function fetchKlines(params: {
  exchange: string
  market_type: 'spot' | 'perps'
  symbol: string
  days: number
}): Promise<any[]> {
  const url = `${API_BASE_URL}${KLINES_ENDPOINT}`
  const { data } = await axios.get<any[]>(url, { params })
  return data
}

export async function fetchReturns(params: {
  exchange: string
  market_type: 'spot' | 'perps'
  symbol: string
  days: number
}): Promise<ReturnPoint[]> {
  const url = `${API_BASE_URL}${RETURNS_ENDPOINT}`
  const { data } = await axios.get<ReturnPoint[]>(url, { params })
  return data.map(d => ({ ts: Number(d.ts), value: Number(d.value) }))
}

export async function fetchVolatility(params: {
  exchange: string
  market_type: 'spot' | 'perps'
  symbol: string
  days: number
  vol_window?: number
}): Promise<VolPoint[]> {
  const url = `${API_BASE_URL}${VOLATILITY_ENDPOINT}`
  const { data } = await axios.get<VolPoint[]>(url, { params })
  return data.map(d => ({ ts: Number(d.ts), value: Number(d.value) }))
}
