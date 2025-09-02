import axios from 'axios'
import { API_BASE_URL, ZSCORE_CROSS_ASSET_ENDPOINT, joinUrl } from '@/utils/constants'

// Create axios instance with base URL
const apiClient = axios.create({
  baseURL: API_BASE_URL,
  timeout: 30000,
  headers: {
    'Accept': 'application/json',
    'Content-Type': 'application/json'
  }
})

// Actual API response structure
export interface CrossAssetMatrixAPIResponse {
  index: string
  correlationMatrix: {
    coins: string[]
    matrix: number[][]
    timestamp: number
  }
  betaMatrix: {
    coins: string[]
    betas: number[]  // Flat array of beta values
  }
  rollingCorr: number[][]
  rollingBeta: number[][]
}

// Our internal types
export interface CrossAssetMatrixResponse {
  index: string
  correlationMatrix: {
    coins: string[]
    matrix: number[][]
    timestamp: number
  }
  betaMatrix: {
    coins: string[]
    betas: number[]  // Keep as flat array
  }
  betaHistogram: {
    buckets: number[]
    counts: number[]
  }
  timeSeriesData: {
    timestamps: number[]
    correlations: { [coin: string]: number[] }
    betas: { [coin: string]: number[] }
  }
  // Add full correlation matrix that includes the index
  fullCorrelationMatrix: {
    coins: string[]
    matrix: number[][]
  }
}

function createBetaHistogram(betas: number[], numBuckets: number = 10): { buckets: number[], counts: number[] } {
  if (betas.length === 0) {
    return { buckets: [], counts: [] }
  }
  
  const min = 0
  const max = Math.max(...betas, 2.5) // Ensure we capture high betas
  const step = (max - min) / numBuckets
  
  const buckets = Array.from({ length: numBuckets + 1 }, (_, i) => 
    Number((min + i * step).toFixed(2))
  )
  
  const counts = new Array(numBuckets).fill(0)
  
  betas.forEach(beta => {
    const bucketIndex = Math.min(
      Math.floor((beta - min) / step),
      numBuckets - 1
    )
    if (bucketIndex >= 0) {
      counts[bucketIndex]++
    }
  })
  
  return { buckets: buckets.slice(0, -1), counts } // Remove last bucket edge
}

export async function fetchCrossAssetMatrix(params: {
  index: string
  compareCoins?: string
  topN?: number
  exchange: string
  marketType: string
  period: string
  window: number
  timeframe?: string
}): Promise<CrossAssetMatrixResponse> {
  try {
    const requestParams: any = {
      index: params.index,
      exchange: params.exchange,
      marketType: params.marketType,
      period: params.period,
      window: params.window,
      timeframe: params.timeframe || '1h'
    }
    
    if (params.topN) {
      requestParams.topN = params.topN
    } else if (params.compareCoins) {
      requestParams.compareCoins = params.compareCoins
    }
    
    console.log('Request URL:', joinUrl(API_BASE_URL, ZSCORE_CROSS_ASSET_ENDPOINT))
    console.log('Request params:', requestParams)
    
    const res = await apiClient.get(ZSCORE_CROSS_ASSET_ENDPOINT, { 
      params: requestParams
    })
    
    console.log('API Response:', res.data)
    
    if (!res.data) {
      throw new Error('Empty API response')
    }
    
    return transformMatrixResponse(res.data)
  } catch (error) {
    if (axios.isAxiosError(error)) {
      console.error('Axios error:', error.response?.status, error.response?.data)
      throw new Error(`API request failed: ${error.response?.status || 'Network error'}`)
    }
    console.error('API Error:', error)
    throw error
  }
}

function transformMatrixResponse(data: CrossAssetMatrixAPIResponse): CrossAssetMatrixResponse {
  // Add validation
  if (!data) {
    throw new Error('Invalid API response: data is null or undefined')
  }
  
  const { index, correlationMatrix, betaMatrix, rollingCorr, rollingBeta } = data
  
  // Find index of the index coin
  const indexCoinIndex = correlationMatrix.coins.findIndex(coin => coin === index)
  
  // Keep the full correlation matrix for the heatmap
  const fullCorrelationMatrix = {
    coins: [...correlationMatrix.coins],
    matrix: correlationMatrix.matrix.map(row => [...row])
  }
  
  // Filter out the index coin for other uses (time series, rankings, etc.)
  const filteredCoins = correlationMatrix.coins.filter(coin => coin !== index)
  const filteredMatrix = correlationMatrix.matrix
    .filter((_, i) => i !== indexCoinIndex)  // Remove index coin row
    .map(row => row.filter((_, j) => j !== indexCoinIndex))  // Remove index coin column
  
  // Filter betas (exclude index coin)
  const filteredBetas = betaMatrix.betas.filter((_, i) => betaMatrix.coins[i] !== index)
  
  // Create beta histogram (already filtered)
  const betaHistogram = createBetaHistogram(filteredBetas)
  
  // Create time series data structure
  const numTimesteps = rollingCorr?.[0]?.length || 0
  const timestamps = Array.from({ length: numTimesteps }, (_, i) => 
    Date.now() - (numTimesteps - i - 1) * 3600000
  )
  
  // Create correlation and beta maps (exclude index coin for time series)
  const correlations: { [coin: string]: number[] } = {}
  const betas: { [coin: string]: number[] } = {}
  
  betaMatrix.coins.forEach((coin, i) => {
    if (coin !== index && rollingCorr && rollingBeta && i < rollingCorr.length && i < rollingBeta.length) {
      correlations[coin] = rollingCorr[i] || []
      betas[coin] = rollingBeta[i] || []
    }
  })
  
  return {
    index,
    correlationMatrix: {
      coins: filteredCoins,
      matrix: filteredMatrix,
      timestamp: correlationMatrix.timestamp
    },
    betaMatrix: {
      coins: filteredCoins,
      betas: filteredBetas
    },
    betaHistogram,
    timeSeriesData: {
      timestamps,
      correlations,
      betas
    },
    fullCorrelationMatrix // Add the full matrix including index
  }
}