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

export interface VolatilityLiquidityResponse {
  volZScoreTimeSeries: VolZScorePoint[]
  volZScoreVsReturns: VolZScoreVsReturn[]
  volumeDistribution: VolumeDistribution
  volumeSummaries: VolumeSummary[]
  volumeSummarySeries: VolumeSummarySeries[]
  spreadSummarySeries?: SpreadSummarySeries[]
}

export async function fetchVolatilityLiquidity(params: {
  coin: string  // This should be a specific coin like 'BTC', 'ETH', etc.
  period: string
  exchange: string
  timeframe: string
  marketType: string
}): Promise<VolatilityLiquidityResponse> {
  try {
    const url = `${API_BASE_URL}/api/statistics/volatility/liquidity`
    
    const apiParams = {
      ...params,
      coin: params.coin || 'BTC'
    }
    
    console.log('Fetching volatility data for single coin:', apiParams)
    
    const { data } = await axios.get<VolatilityLiquidityResponse>(url, { 
      params: apiParams 
    })
    
    console.log(`API returned ${data.volZScoreTimeSeries?.length || 0} data points for ${params.coin}`)
    
    return data
  } catch (error) {
    console.error('Error fetching volatility liquidity data:', error)
    handleApiError(error)
    throw error
  }
}

// New helper function to fetch data for multiple coins if needed
export async function fetchVolatilityLiquidityMultiple(params: {
  coins: string[]  // Array of specific coins
  period: string
  exchange: string
  timeframe: string
  marketType: string
}): Promise<Record<string, VolatilityLiquidityResponse>> {
  try {
    const promises = params.coins.map(coin =>
      fetchVolatilityLiquidity({
        ...params,
        coin
      }).then(data => ({ coin, data }))
    )
    
    const results = await Promise.all(promises)
    
    return results.reduce((acc, { coin, data }) => {
      acc[coin] = data
      return acc
    }, {} as Record<string, VolatilityLiquidityResponse>)
  } catch (error) {
    console.error('Error fetching multiple volatility liquidity data:', error)
    throw error
  }
}