// src/api/meta.ts
import axios from 'axios'
import { API_BASE_URL, OPTIONS_ENDPOINT, joinUrl } from '@/utils/constants'
import { handleApiError } from './error'

export type MarketType = 'spot' | 'perps'

export interface MetaData {
  exchanges: string[]
  coins: string[]          
}

export async function fetchMetaData(market_type: string, quote: string) {
  try {
    const url = joinUrl(API_BASE_URL, OPTIONS_ENDPOINT || '/api/meta-data/options')
    const { data } = await axios.get(url, { params: { market_type, quote } })
    return data as { coins: string[]; exchanges: string[] }
  } catch (e) {
    throw new Error(handleApiError(e, 'fetchMetaData'))
  }
}