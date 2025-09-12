// src/api/zscore/volatilityDynamics.ts
import axios from 'axios'
import { API_BASE_URL, joinUrl } from '@/utils/constants'

// Create axios instance
const apiClient = axios.create({
  baseURL: API_BASE_URL,
  timeout: 30000,
  headers: {
    'Accept': 'application/json',
    'Content-Type': 'application/json'
  }
})

// Define the endpoint
export const VOLATILITY_DYNAMICS_ENDPOINT = '/api/zscore/volatility-dynamics'

export interface TimeSeriesPoint {
  timestamp: number
  value: number
}

export interface TimeSeriesData {
  symbol: string
  data: TimeSeriesPoint[]
}

export interface SkewKurtosisData {
  symbol: string
  skewness: number
  kurtosis: number
}

export interface RegimeData {
  symbol: string
  regime: 'normal' | 'stressed' | 'euphoric' | 'compressed' | 'unstable'
  volatility: number
  skewness: number
  kurtosis: number
}

export interface InstabilityData {
  symbol: string
  vov: number
  skewness: number
  status: 'critical' | 'warning' | 'normal'
  status_label: string
}

export interface FlowVolBeta {
  symbol: string
  beta: number
  r_squared: number
}

export interface VolStats {
  symbol: string
  current_vol: number
  avg_vol: number
  percentile: number
  skewness: number
  kurtosis: number
  vov: number
}

export interface VolDistribution {
  bins: number[]
  frequencies: number[]
  normalCurve?: number[]
}

export interface DistributionStat {
  label: string
  value: number
  line: boolean
  color?: string
}

export interface VolatilityDynamicsResponse {
  vovTimeSeries: TimeSeriesData[]
  skewnessTimeSeries: TimeSeriesData[]
  skewKurtosisScatter: SkewKurtosisData[]
  volDistribution: VolDistribution
  distributionStats: DistributionStat[]
  covarianceTimeSeries: TimeSeriesData[]
  regimeClassification: RegimeData[]
  instabilityRankings: InstabilityData[]
  flowVolBeta: FlowVolBeta[]
  volStatsSummary: VolStats[]
}

export interface VolatilityDynamicsParams {
  exchange?: string
  marketType?: string
  period: string
  timeframe?: string
  topN?: number
  volWindow?: number
  vovWindow?: number
  momentWindow?: number
  histogramCoin?: string
  binStepPct?: number
}

export async function fetchVolatilityDynamics(params: VolatilityDynamicsParams): Promise<VolatilityDynamicsResponse> {
  try {
    const requestParams = {
      topN: params.topN || 12,
      exchange: params.exchange || 'binance',
      marketType: params.marketType || 'spot',
      period: params.period.replace('d', ''), // Convert "30d" to "30"
      timeframe: params.timeframe || '1h',
      volWindow: params.volWindow || 24,
      vovWindow: params.vovWindow || 96,
      momentWindow: params.momentWindow || 48,
      histogramCoin: params.histogramCoin || 'BTC',
      binStepPct: params.binStepPct || 5
    }

    console.log('Fetching volatility dynamics:', requestParams)

    const response = await apiClient.get(VOLATILITY_DYNAMICS_ENDPOINT, {
      params: requestParams
    })

    if (!response.data) {
      throw new Error('Empty response from API')
    }

    return response.data
  } catch (error) {
    if (axios.isAxiosError(error)) {
      console.error('API Error:', error.response?.status, error.response?.data)
      throw new Error(
        error.response?.data?.error || 
        `API request failed: ${error.response?.status || 'Network error'}`
      )
    }
    throw error
  }
}