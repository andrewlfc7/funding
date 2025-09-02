import axios from 'axios'
import { API_BASE_URL, LEADERS_LAGGARDS_ENDPOINT } from '@/utils/constants'

const apiClient = axios.create({
  baseURL: API_BASE_URL,
  timeout: 30000,
  headers: {
    'Accept': 'application/json',
    'Content-Type': 'application/json'
  }
})

export interface LeadersLaggardsParams {
  exchange: string
  marketType: string
  timeframe: string
  period: string
  topN: number
}

export interface CoinRanking {
  symbol: string
  zscore: number
  returns: number
  volume: number
  rank: number
}

export interface VolumeSpike {
  symbol: string
  volumeZScore: number
  priceChange: number
}

export interface DecorrelatedAsset {
  symbol: string
  correlationWithMarket: number
  avgCorrelation: number
}

export interface LeadLagMatrix {
  coins: string[]
  lags: number[]
  matrix: number[][][]
}

export interface LeadersLaggardsResponse {
  leaders: CoinRanking[]
  laggards: CoinRanking[]
  volumeSpikes: VolumeSpike[]
  decorrelated: DecorrelatedAsset[]
  leadLagMatrix: LeadLagMatrix
}

export async function fetchLeadersLaggards(params: LeadersLaggardsParams): Promise<LeadersLaggardsResponse> {
  try {
    const response = await apiClient.get(LEADERS_LAGGARDS_ENDPOINT, {
      params: {
        exchange: params.exchange,
        marketType: params.marketType,
        timeframe: params.timeframe,
        period: params.period,
        topN: params.topN
      }
    })
    
    return response.data
  } catch (error) {
    console.error('Error fetching leaders/laggards:', error)
    throw error
  }
}