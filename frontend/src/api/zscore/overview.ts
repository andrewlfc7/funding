import axios from 'axios'
import { API_BASE_URL, ZSCORE_OVERVIEW_ENDPOINT, joinUrl } from '@/utils/constants'
import { handleApiError } from '@/api/error'
import type { MarketType, Timeframe, ZScoreOverviewResponse } from './types'

export async function fetchZScoreOverview(params: {
  exchange: string
  timeframe: Timeframe
  period: string
  marketType?: MarketType
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
