import axios from 'axios'
import { API_BASE_URL, MARKET_REGIME_ENDPOINT, joinUrl } from '@/utils/constants'
import { handleApiError } from '@/api/error'
import type { MarketType, MarketRegimeResponse } from './types'

export async function fetchMarketRegime(params: {
  exchange: string
  period: string
  marketType?: MarketType
  topN?: number
}): Promise<MarketRegimeResponse> {
  try {
    const url = joinUrl(API_BASE_URL, MARKET_REGIME_ENDPOINT)
    const { data } = await axios.get<MarketRegimeResponse>(url, {
      params: { marketType: 'spot', ...params },
    })
    return data
  } catch (err) {
    throw handleApiError(err)
  }
}
