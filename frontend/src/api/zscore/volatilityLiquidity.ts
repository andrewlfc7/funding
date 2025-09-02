// src/api/zscore/volatilityLiquidity.ts
import axios from 'axios'
import { API_BASE_URL } from '@/utils/constants'
import { handleApiError } from '@/api/error'

export interface VolZScorePoint {
  timestamp: number
  volZScore: number
  threshold2Sigma: number
  thresholdNeg2Sigma: number
  symbol: string
}

export interface VolZScoreVsReturn {
  symbol: string
  volZScore: number
  dailyRange: number
  volume: number
  returns?: number
}

export interface VolumeDistribution {
  buckets: number[]
  counts: number[]
  currentValue?: number
}

export interface VolumeSummary {
  symbol: string
  weeklyDollarVolume: number
  avgDailyDollarVolume: number
  avgHourlyDollarVolume: number
  currentDollarVolume: number
  ratioCurrentToAvgDaily: number
  ratioEWMAToAvgDaily: number
}

export interface VolumeSummaryPoint {
  timestamp: number
  weeklyDollarVolume: number
  avgDailyDollarVolume: number
  avgHourlyDollarVolume: number
  ewmaDollarVolume: number
  dollarVolume: number
}

export interface VolumeSummarySeries {
  symbol: string
  series: VolumeSummaryPoint[]
}


export async function fetchVolatilityLiquidity(params: {
  coin: string
  period: string
  exchange: string
  marketType: string
}): Promise<VolatilityLiquidityResponse> {
  try {
    const url = `${API_BASE_URL}/api/statistics/volatility/liquidity`
    const { data } = await axios.get<VolatilityLiquidityResponse>(url, { params })
    return data
  } catch (error) {
    handleApiError(error)
    throw error
  }
}




// src/api/zscore/volatilityLiquidity.ts
// Add these new interfaces
export interface SpreadSummaryPoint {
  timestamp: number
  spread1h: number
  avg1d: number
  std1d: number
  avg7d: number
  std7d: number
  avgZScore: number
}

export interface SpreadSummarySeries {
  symbol: string
  series: SpreadSummaryPoint[]
}

// Update the main response interface
export interface VolatilityLiquidityResponse {
  volZScoreTimeSeries: VolZScorePoint[]
  volZScoreVsReturns: VolZScoreVsReturn[]
  volumeDistribution: VolumeDistribution
  volumeSummaries: VolumeSummary[]
  volumeSummarySeries: VolumeSummarySeries[]
  spreadSummarySeries?: SpreadSummarySeries[]  // Add this
}