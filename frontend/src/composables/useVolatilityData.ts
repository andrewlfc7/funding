import { ref, computed } from 'vue'
import { 
  fetchVolatilityAnalysis, 
  transformVolatilityToTimeSeries,
  groupDataBySymbol,
  getLatestBySymbol,
  type VolatilityDataPoint 
} from '@/api/zscore'

export function useVolatilityData() {
  // State
  const rawData = ref<VolatilityDataPoint[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  // Group data by symbol
  const dataBySymbol = computed(() => groupDataBySymbol(rawData.value))
  const latestBySymbol = computed(() => getLatestBySymbol(rawData.value))
  
  // Transform data for charts
  const chartData = computed(() => transformVolatilityToTimeSeries(rawData.value))
  
  // For Dashboard 2: Volatility Analysis plots
  
  // 1. Vol Z-Score vs Returns
  const volZScoreVsReturns = computed(() => {
    return Array.from(latestBySymbol.value.entries()).map(([symbol, data]) => ({
      symbol,
      returns: data.returns * 100,
      volatilityZScore: data.volatilityZScore
    }))
  })
  
  // 2. Daily Range Distribution
  const rangeDistribution = computed(() => {
    const allRanges = Array.from(latestBySymbol.value.values()).map(d => d.range * 100)
    
    if (allRanges.length === 0) return null
    
    const buckets = [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20]
    const counts = new Array(buckets.length - 1).fill(0)
    
    allRanges.forEach(range => {
      const bucketIndex = buckets.findIndex((b, i) => 
        range >= b && (i === buckets.length - 1 || range < buckets[i + 1])
      ) - 1
      if (bucketIndex >= 0 && bucketIndex < counts.length) {
        counts[bucketIndex]++
      }
    })
    
    return { 
      buckets: buckets.slice(0, -1).map((b, i) => `${b}-${buckets[i+1]}%`),
      counts
    }
  })
  
  // 3. Volume Z-Score vs Daily Range (need to add volume z-score to backend data)
  const volumeZScoreVsRange = computed(() => {
    return Array.from(latestBySymbol.value.entries()).map(([symbol, data]) => ({
      symbol,
      range: data.range * 100,
      volumeZScore: 0, // TODO: Backend needs to provide this
      volatilityZScore: data.volatilityZScore
    }))
  })
  
  // 4. High/Low volatility coins
  const highVolCoins = computed(() => 
    Array.from(latestBySymbol.value.entries())
      .filter(([_, data]) => data.volatilityZScore > 1.5)
      .sort((a, b) => b[1].volatilityZScore - a[1].volatilityZScore)
      .slice(0, 10)
      .map(([symbol, data]) => ({
        symbol,
        volatility: data.volatility * 100,
        volatilityZScore: data.volatilityZScore,
        range: data.range * 100,
        returns: data.returns * 100
      }))
  )
  
  const lowVolCoins = computed(() => 
    Array.from(latestBySymbol.value.entries())
      .filter(([_, data]) => data.volatilityZScore < -1.5)
      .sort((a, b) => a[1].volatilityZScore - b[1].volatilityZScore)
      .slice(0, 10)
      .map(([symbol, data]) => ({
        symbol,
        volatility: data.volatility * 100,
        volatilityZScore: data.volatilityZScore,
        range: data.range * 100,
        returns: data.returns * 100
      }))
  )
  
  // 5. Realized Vol Percentiles (need backend support)
  const realizedVolPercentiles = computed(() => {
    // For now, calculate simple percentiles based on current data
    const sortedVols = Array.from(latestBySymbol.value.values())
      .sort((a, b) => a.volatility - b.volatility)
    
    return Array.from(latestBySymbol.value.entries()).map(([symbol, data]) => {
      const rank = sortedVols.findIndex(v => v.volatility >= data.volatility)
      const percentile = (rank / sortedVols.length) * 100
      
      return {
        symbol,
        volatility: data.volatility * 100,
        volatilityZScore: data.volatilityZScore,
        percentile
      }
    })
  })
  
  // Aggregate statistics
  const stats = computed(() => {
    const allVolatilities = Array.from(latestBySymbol.value.values()).map(d => d.volatility)
    
    if (allVolatilities.length === 0) {
      return { 
        meanVolatility: 0, 
        maxVolatility: 0,
        minVolatility: 0,
        highVolCount: 0,
        lowVolCount: 0
      }
    }
    
    return { 
      meanVolatility: (allVolatilities.reduce((a, b) => a + b, 0) / allVolatilities.length) * 100,
      maxVolatility: Math.max(...allVolatilities) * 100,
      minVolatility: Math.min(...allVolatilities) * 100,
      highVolCount: highVolCoins.value.length,
      lowVolCount: lowVolCoins.value.length
    }
  })
  
  // Fetch data
  async function fetchData(params: {
    timeframe: string
    period: string
    exchange: string
    topN?: number
  }) {
    loading.value = true
    error.value = null
    
    try {
      const response = await fetchVolatilityAnalysis({
        ...params,
        topN: params.topN || 50
      })
      rawData.value = response.volatilityTimeSeries || []
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch volatility data'
      console.error('Volatility fetch error:', e)
    } finally {
      loading.value = false
    }
  }
  
  function clear() {
    rawData.value = []
    error.value = null
  }
  
  return {
    // State
    rawData,
    loading,
    error,
    
    // Grouped data
    dataBySymbol,
    latestBySymbol,
    chartData,
    
    // Computed plots data
    volZScoreVsReturns,
    rangeDistribution,
    volumeZScoreVsRange,
    realizedVolPercentiles,
    highVolCoins,
    lowVolCoins,
    stats,
    
    // Methods
    fetchData,
    clear,
    hasData: computed(() => rawData.value.length > 0)
  }
}