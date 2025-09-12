// src/api/zscore/marketSeasonality.ts
import axios from 'axios'
import { API_BASE_URL, ZSCORE_MARKET_SEASONALITY_ENDPOINT, joinUrl } from '@/utils/constants'

// Create axios instance
const apiClient = axios.create({
  baseURL: API_BASE_URL,
  timeout: 30000,
  headers: {
    'Accept': 'application/json',
    'Content-Type': 'application/json'
  }
})

// Types
export interface SeasonalityAnomaly {
  symbol: string
  period: string
  metric: string
  current: number
  historical: number
  zscore: number
}

export interface VolumePersistence {
  symbol: string
  lag1h: number
  lag4h: number
  lag24h: number
  pattern: string
}

export interface ActivityPeriod {
  id: string
  asset: string
  period: string
  volatility: number
  volume: number
}

export interface Pattern {
  id: string
  name: string
  strength: number
}

export interface VolatilityCluster {
  id: string
  period: string
  assets: string[]
}

export interface WeekendEffect {
  asset: string
  effect: number
}

export interface TimezoneEffect {
  zone: string
  activeHours: string
  impact: string
}

export interface MarketSeasonalityResponse {
  intradayHeatmap: number[][]
  weekdayHeatmap: {
    volatility: number[][]
    volume: number[][]
    returns: number[][]
  }
  monthlySeasonality: {
    [asset: string]: {
      returns: number[]
      volatility: number[]
    }
  }
  volumeAutocorrelation: {
    [asset: string]: number[]
  }
  anomalies: SeasonalityAnomaly[]
  volumePersistence: VolumePersistence[]
  mostActivePeriods: ActivityPeriod[]
  leastActivePeriods: ActivityPeriod[]
  strongestPatterns: Pattern[]
  volatilityClusters: VolatilityCluster[]
  weekendEffect: WeekendEffect[]
  timezoneEffects: TimezoneEffect[]
}

export interface MarketSeasonalityParams {
  exchange?: string
  marketType?: string
  period: string
  timeframe?: string
  topN?: number
}

export async function fetchMarketSeasonality(params: MarketSeasonalityParams): Promise<MarketSeasonalityResponse> {
  try {
    const requestParams = {
      exchange: params.exchange || 'binance',
      marketType: params.marketType || 'spot',
      period: params.period,
      timeframe: params.timeframe || '1h',
      topN: params.topN || 10
    }

    console.log('Fetching market seasonality:', requestParams)

    const response = await apiClient.get(ZSCORE_MARKET_SEASONALITY_ENDPOINT, {
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