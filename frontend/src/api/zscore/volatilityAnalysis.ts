import axios from 'axios'
import { API_BASE_URL, ZSCORE_VOLATILITY_ENDPOINT, joinUrl } from '@/utils/constants'
import { handleApiError } from '@/api/error'
import type { MarketType, Timeframe, VolatilityAnalysisResponse } from './types'

export async function fetchVolatilityAnalysis(params: {
  exchange: string
  timeframe: Timeframe
  period: string
  marketType?: MarketType
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
