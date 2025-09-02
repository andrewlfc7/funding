import { ref, computed, watch } from 'vue'
import { fetchInterAssetZScore, type InterAssetZScoreResponse } from '@/api/zscore/interAsset'

export function useInterAssetZScore() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  // Request params
  const period = ref('30d')
  const exchange = ref('binance')
  const marketType = ref<'spot' | 'perps'>('spot')
  const topN = ref(20)
  const window = ref(20)
  const timeframe = ref<'1h' | '4h' | '1d'>('1d')
  const indexCoin = ref('BTC')
  
  // Response data
  const rawData = ref<InterAssetZScoreResponse | null>(null)
  
  // Computed data for components
  const zscoreCorrelationMatrix = computed(() => {
    if (!rawData.value?.zscoreCorrelationMatrix) return null
    const { coins, matrix } = rawData.value.zscoreCorrelationMatrix
    return {
      labels: coins,
      data: matrix
    }
  })
  
  const zscoreBetaMatrix = computed(() => {
    if (!rawData.value?.zscoreBetaMatrix) return null
    const { coins, matrix } = rawData.value.zscoreBetaMatrix
    return {
      labels: coins,
      data: matrix
    }
  })
  
  const divergenceData = computed(() => {
    return rawData.value?.pairDivergence || []
  })
  
  // Get list of available coins from the correlation matrix
  const availableCoins = computed(() => {
    return rawData.value?.zscoreCorrelationMatrix?.coins || []
  })
  
  // Find top relationships from correlation matrix
  const topRelationships = computed(() => {
    if (!rawData.value?.zscoreCorrelationMatrix) return { positive: [], negative: [] }
    
    const { coins, matrix } = rawData.value.zscoreCorrelationMatrix
    const relationships: Array<{
      coin1: string
      coin2: string
      correlation: number
      beta: number
    }> = []
    
    // Extract upper triangle of correlation matrix (avoid duplicates)
    for (let i = 0; i < coins.length; i++) {
      for (let j = i + 1; j < coins.length; j++) {
        const correlation = matrix[i][j]
        const beta = rawData.value.zscoreBetaMatrix?.matrix[i][j] || 0
        
        relationships.push({
          coin1: coins[i],
          coin2: coins[j],
          correlation,
          beta
        })
      }
    }
    
    // Sort by absolute correlation
    relationships.sort((a, b) => Math.abs(b.correlation) - Math.abs(a.correlation))
    
    // Get top positive and negative correlations
    const positive = relationships
      .filter(r => r.correlation > 0)
      .slice(0, 5)
    
    const negative = relationships
      .filter(r => r.correlation < 0)
      .slice(0, 5)
    
    return { positive, negative }
  })
  
  // Find most independent coins (lowest average correlation)
  const independentCoins = computed(() => {
    if (!rawData.value?.zscoreCorrelationMatrix) return []
    
    const { coins, matrix } = rawData.value.zscoreCorrelationMatrix
    const avgCorrelations = coins.map((coin, i) => {
      // Calculate average absolute correlation with other coins
      const correlations = matrix[i].filter((_, j) => i !== j)
      const avgCorr = correlations.reduce((sum, corr) => sum + Math.abs(corr), 0) / correlations.length
      
      return {
        symbol: coin,
        avgCorrelation: avgCorr,
        independenceScore: (1 - avgCorr) * 10 // Convert to 0-10 scale
      }
    })
    
    // Sort by independence score (highest first)
    return avgCorrelations
      .sort((a, b) => b.independenceScore - a.independenceScore)
      .slice(0, 5)
  })
  
  const fetchData = async () => {
    loading.value = true
    error.value = null
    
    try {
      const data = await fetchInterAssetZScore({
        period: period.value,
        exchange: exchange.value,
        marketType: marketType.value,
        topN: topN.value,
        window: window.value,
        timeframe: timeframe.value,
        indexCoin: indexCoin.value
      })
      
      rawData.value = data
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch inter-asset data'
      console.error('Inter-asset fetch error:', e)
    } finally {
      loading.value = false
    }
  }
  
  // Auto-refresh on param changes
  watch([period, exchange, marketType, topN, window, timeframe, indexCoin], () => {
    fetchData()
  })
  
  return {
    // State
    loading,
    error,
    
    // Params
    period,
    exchange,
    marketType,
    topN,
    window,
    timeframe,
    indexCoin,
    
    // Data
    zscoreCorrelationMatrix,
    zscoreBetaMatrix,
    divergenceData,
    availableCoins,
    topRelationships,
    independentCoins,
    
    // Methods
    fetchData
  }
}