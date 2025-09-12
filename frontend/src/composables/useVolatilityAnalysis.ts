// frontend/src/composables/useVolatilityAnalysis.ts
import { ref, computed } from 'vue'
import { 
  fetchVolatilityAnalysis,
  type VolatilityDataPoint,
  type VolatilityAnalysisResponse,
  type Timeframe
} from '@/api/zscore'

export function useVolatilityData() {
  // State
  const rawData = ref<VolatilityAnalysisResponse | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  // Constants for annualization
  const ANNUALIZATION_FACTOR = {
    '1h': Math.sqrt(24 * 365),  // Hourly to annual
    '4h': Math.sqrt(6 * 365),   // 4-hourly to annual
    '1d': Math.sqrt(365)        // Daily to annual
  }
  
  // Group volatility time series data by symbol
  const dataBySymbol = computed(() => {
    if (!rawData.value?.volatilityTimeSeries) return new Map()
    const grouped = new Map<string, VolatilityDataPoint[]>()
    rawData.value.volatilityTimeSeries.forEach(point => {
      const list = grouped.get(point.symbol) || []
      list.push(point)
      grouped.set(point.symbol, list)
    })
    return grouped
  })
  
  // Get the latest data point for each symbol
  const latestBySymbol = computed(() => {
    const latest = new Map<string, VolatilityDataPoint>()
    dataBySymbol.value.forEach((points, symbol) => {
      if (points.length > 0) {
        latest.set(symbol, points[points.length - 1])
      }
    })
    return latest
  })
  
  // Calculate volume statistics for z-score calculation
  const volumeStats = computed(() => {
    if (!rawData.value?.volatilityTimeSeries) return { mean: 0, std: 0 }
    
    const validVolumes = rawData.value.volatilityTimeSeries
      .map(d => d.volume)
      .filter((v): v is number => v !== undefined && v !== null && v > 0)
    
    if (validVolumes.length === 0) return { mean: 0, std: 0 }
    
    const mean = validVolumes.reduce((a, b) => a + b, 0) / validVolumes.length
    const variance = validVolumes.reduce((sum, v) => sum + Math.pow(v - mean, 2), 0) / validVolumes.length
    const std = Math.sqrt(variance)
    
    return { mean, std }
  })
  
  // Calculate volume z-score for a given volume
  const calculateVolumeZScore = (volume: number | undefined): number => {
    if (!volume || volumeStats.value.std === 0) return 0
    return (volume - volumeStats.value.mean) / volumeStats.value.std
  }
  
  // Annualized volatility data for heatmap
  const annualizedVolatilityData = computed(() => {
    if (!latestBySymbol.value.size) return []
    
    const currentTimeframe = rawData.value?.volatilityTimeSeries[0]?.symbol ? '1h' : '1h' // Default to 1h
    const annualizationFactor = ANNUALIZATION_FACTOR[currentTimeframe as keyof typeof ANNUALIZATION_FACTOR]
    
    const data = Array.from(latestBySymbol.value.entries()).map(([symbol, point]) => ({
      symbol,
      volatility: point.volatility * 100,
      annualizedVol: point.volatility * 100 * annualizationFactor,
      volatilityZScore: point.volatilityZScore,
      volumeZScore: calculateVolumeZScore(point.volume),
      returns: point.returns * 100,
      volume: point.volume || 0
    }))
    
    // Sort by annualized volatility descending and add rank
    return data
      .sort((a, b) => b.annualizedVol - a.annualizedVol)
      .map((item, index) => ({ ...item, rank: index }))
  })
  
  // 1-Hour Volatility vs Volume Z-Score
  const volVsVolumeZScore1H = computed(() => {
    if (!latestBySymbol.value.size) return []
    
    // For 1H volatility, use the raw volatility if timeframe is 1h
    // Otherwise, scale appropriately
    return Array.from(latestBySymbol.value.entries()).map(([symbol, point]) => ({
      symbol,
      volatility: point.volatility * 100, // Already in 1H if timeframe is 1h
      volumeZScore: calculateVolumeZScore(point.volume),
      volatilityZScore: point.volatilityZScore
    }))
  })
  
  // 1-Day Volatility vs Volume Z-Score
  const volVsVolumeZScore1D = computed(() => {
    if (!latestBySymbol.value.size) return []
    
    // Scale to daily volatility
    const scaleFactor = Math.sqrt(24) // 1H to 1D scaling
    
    return Array.from(latestBySymbol.value.entries()).map(([symbol, point]) => ({
      symbol,
      volatility: point.volatility * 100 * scaleFactor, // Scale to daily
      volumeZScore: calculateVolumeZScore(point.volume),
      volatilityZScore: point.volatilityZScore
    }))
  })
  

const volumeZScoreVsReturns = computed(() => {
  if (!rawData.value?.volVsReturns) return []
  
  const symbolMap = new Map<string, VolatilityDataPoint>()
  latestBySymbol.value.forEach((point, symbol) => {
    symbolMap.set(symbol, point)
  })
  
  return rawData.value.volVsReturns.map(d => {
    // Find the corresponding symbol data
    const symbolData = Array.from(latestBySymbol.value.entries()).find(
      ([symbol, point]) => point.returns === d.returns && point.volume === d.volume
    )
    
    return {
      symbol: symbolData?.[0] || 'Unknown', // Use actual symbol or fallback
      returns: d.returns * 100,
      volumeZScore: calculateVolumeZScore(d.volume),
      volatilityZScore: d.volZScore,
      volume: d.volume
    }
  }).filter(d => d.symbol !== 'Unknown') // Filter out unknown symbols
})




  // High volatility coins (Z > 1.5)
  const highVolCoins = computed(() => {
    return annualizedVolatilityData.value
      .filter(coin => coin.volatilityZScore > 1.5)
      .slice(0, 10)
  })
  
  // Low volatility coins (Z < -1.5)
  const lowVolCoins = computed(() => {
    return annualizedVolatilityData.value
      .filter(coin => coin.volatilityZScore < -1.5)
      .sort((a, b) => a.volatilityZScore - b.volatilityZScore)
      .slice(0, 10)
  })
  

  function getVolatilityHeatmapColor(annualizedVol: number): string {
    const clampedVol = Math.min(Math.max(annualizedVol, 0), 100);
    const ratio = clampedVol / 100;

    if (ratio < 0.5) {
      // Green to Gray
      const intensity = ratio * 2;
      // interpolate between #319755 and #5a504d
      const r = Math.floor(0x31 + (0x5a - 0x31) * intensity);
      const g = Math.floor(0x97 + (0x50 - 0x97) * intensity);
      const b = Math.floor(0x55 + (0x4d - 0x55) * intensity);
      return `rgb(${r}, ${g}, ${b})`;
    } else {
      // Gray to Red
      const intensity = (ratio - 0.5) * 2;
      // interpolate between #5a504d and #8d1b1b
      const r = Math.floor(0x5a + (0x8d - 0x5a) * intensity);
      const g = Math.floor(0x50 + (0x1b - 0x50) * intensity);
      const b = Math.floor(0x4d + (0x1b - 0x4d) * intensity);
      return `rgb(${r}, ${g}, ${b})`;
    }
  }



  // Fetch data
  async function fetchData(params: {
    timeframe: string
    period: string
    exchange: string
    topN?: number
    coin?: string
  }) {
    loading.value = true
    error.value = null
    
    try {
      const response = await fetchVolatilityAnalysis({
        ...params,
        timeframe: params.timeframe as Timeframe,
        topN: params.topN || 50
      })
      rawData.value = response
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch volatility data'
      console.error('Volatility fetch error:', e)
    } finally {
      loading.value = false
    }
  }
  
  function clear() {
    rawData.value = null
    error.value = null
  }
  
  return {
    // State
    loading,
    error,
    
    // Computed data
    annualizedVolatilityData,
    volVsVolumeZScore1H,
    volVsVolumeZScore1D,
    volumeZScoreVsReturns,
    highVolCoins,
    lowVolCoins,
    
    // Helper functions
    getVolatilityHeatmapColor,
    
    // Methods
    fetchData,
    clear,
    hasData: computed(() => rawData.value !== null && rawData.value.volatilityTimeSeries.length > 0)
  }
}