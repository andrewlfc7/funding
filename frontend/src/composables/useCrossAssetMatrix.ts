import { ref, computed } from 'vue'
import { 
  fetchCrossAssetMatrix,
  type CrossAssetMatrixResponse
} from '@/api/zscore/crossAsset'

export function useCrossAssetMatrix() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  const matrixData = ref<CrossAssetMatrixResponse | null>(null)

  // Computed values
  const correlationRankings = computed(() => {
    if (!matrixData.value) return { top: [], bottom: [] }
    
    const coins = matrixData.value.betaMatrix.coins
    const betas = matrixData.value.betaMatrix.betas
    const corrMatrix = matrixData.value.correlationMatrix.matrix
    
    // Get correlations with index (first column)
    const rankings = coins
      .map((coin, i) => ({
        symbol: coin,
        correlation: corrMatrix[i]?.[0] || 0,
        beta: betas[i]
      }))
      .sort((a, b) => b.correlation - a.correlation)
    
    return {
      top: rankings.slice(0, 5).map((item, i) => ({ ...item, rank: i + 1 })),
      bottom: rankings.slice(-5).reverse().map((item, i) => ({ ...item, rank: i + 1 }))
    }
  })

  const stats = computed(() => {
    if (!matrixData.value) {
      return {
        meanCorr: 0,
        medianCorr: 0,
        stdCorr: 0,
        meanBeta: 0,
        medianBeta: 0,
        highBetaCount: 0
      }
    }

    const correlations = matrixData.value.correlationMatrix.matrix
      .map(row => row[0]) // Get correlation with index (first column)

    const betas = matrixData.value.betaMatrix.betas

    return {
      meanCorr: calculateMean(correlations),
      medianCorr: calculateMedian(correlations),
      stdCorr: calculateStd(correlations),
      meanBeta: calculateMean(betas),
      medianBeta: calculateMedian(betas),
      highBetaCount: betas.filter(b => b > 1).length
    }
  })

  // Beta highlights for histogram
  const betaHighlights = computed(() => {
    if (!matrixData.value) return []
    
    return matrixData.value.betaMatrix.coins
      .map((coin, i) => ({
        value: matrixData.value!.betaMatrix.betas[i],
        label: coin
      }))
  })

  const timeSeriesData = computed(() => {
    if (!matrixData.value || !matrixData.value.timeSeriesData) {
      return { correlations: null, betas: null }
    }
    
    const { timestamps, correlations, betas } = matrixData.value.timeSeriesData
    
    // Format for TimeSeriesChart component
    const correlationSeries = Object.entries(correlations)
      .map(([coin, values]) => ({
        symbol: coin,
        data: values.map((value, i) => ({
          timestamp: timestamps[i],
          value: value
        }))
      }))
    
    const betaSeries = Object.entries(betas)
      .map(([coin, values]) => ({
        symbol: coin,
        data: values.map((value, i) => ({
          timestamp: timestamps[i],
          value: value
        }))
      }))
    
    return {
      correlations: correlationSeries,
      betas: betaSeries
    }
  })

  // Full correlation matrix for heatmap - INCLUDES all coins including index
  const fullCorrelationMatrix = computed(() => {
    if (!matrixData.value || !matrixData.value.fullCorrelationMatrix) return null
    
    // Return the full matrix from the API including the index coin
    return {
      labels: matrixData.value.fullCorrelationMatrix.coins,
      data: matrixData.value.fullCorrelationMatrix.matrix
    }
  })

  // Full beta matrix for heatmap - ADD THIS
  const fullBetaMatrix = computed(() => {
    if (!matrixData.value || !matrixData.value.betaMatrix) return null
    
    const coins = matrixData.value.betaMatrix.coins
    const betas = matrixData.value.betaMatrix.betas
    
    // Create a matrix where each row represents a coin and contains its beta values
    // For a beta matrix, we typically show beta of each coin vs others
    // Since beta is typically calculated vs an index, we'll create a diagonal matrix
    // or show betas vs the index coin
    const n = coins.length
    const matrix: number[][] = []
    
    // Create a matrix where each row i shows the beta of coin i
    // Since betas array contains beta vs index, we'll replicate this across columns
    for (let i = 0; i < n; i++) {
      const row = new Array(n).fill(0)
      // Set diagonal to the beta value
      row[i] = betas[i]
      // Or you could set all values in the row to the same beta
      // for (let j = 0; j < n; j++) {
      //   row[j] = betas[i]
      // }
      matrix.push(row)
    }
    
    // Alternative: If you want to show betas as a single column repeated
    // This might be more appropriate for visualization
    const betaMatrix: number[][] = []
    for (let i = 0; i < n; i++) {
      const row = []
      for (let j = 0; j < n; j++) {
        // Each cell shows the beta of coin i
        row.push(betas[i])
      }
      betaMatrix.push(row)
    }
    
    return {
      labels: coins,
      data: betaMatrix
    }
  })

  async function load(params: {
    index?: string
    topN?: number
    exchange?: string
    marketType?: string
    period: string
    window: number
    timeframe?: string
  }) {
    loading.value = true
    error.value = null
    
    try {
      const requestParams = {
        index: params.index || 'BTC',
        exchange: params.exchange || 'binance',
        marketType: params.marketType || 'spot',
        period: params.period,
        window: params.window,
        timeframe: params.timeframe || '1h',
        topN: params.topN || 10
      }
      
      matrixData.value = await fetchCrossAssetMatrix(requestParams)
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to load data'
      console.error('Cross-asset data error:', e)
    } finally {
      loading.value = false
    }
  }
  
  return { 

    loading, 
    error, 
    data: matrixData,
    timeSeriesData,         // Time series data excluding index
    correlationRankings,    // Rankings excluding index  
    betaHighlights,         // Beta highlights excluding index
    stats,                  // Stats excluding index
    fullCorrelationMatrix,  // Full matrix including index from API
    fullBetaMatrix,         // Beta matrix for heatmap visualization
    load 
  }
}

function calculateMean(numbers: number[]): number {
  if (numbers.length === 0) return 0
  return numbers.reduce((a, b) => a + b, 0) / numbers.length
}

function calculateMedian(numbers: number[]): number {
  if (numbers.length === 0) return 0
  const sorted = [...numbers].sort((a, b) => a - b)
  const mid = Math.floor(sorted.length / 2)
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2
}

function calculateStd(numbers: number[]): number {
  if (numbers.length === 0) return 0
  const mean = calculateMean(numbers)
  const variance = numbers.reduce((sum, num) => sum + Math.pow(num - mean, 2), 0) / numbers.length
  return Math.sqrt(variance)
}