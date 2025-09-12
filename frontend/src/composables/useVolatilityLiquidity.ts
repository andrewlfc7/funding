

import { ref, computed } from 'vue'
import { 
  fetchVolatilityLiquidity, 
  type VolatilityLiquidityResponse,
  type VolumeSummaryPoint 
} from '@/api/zscore/volatilityLiquidity'

export function useVolatilityLiquidity() {
  // State
  const data = ref<VolatilityLiquidityResponse | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  // Computed values from API data
  const volZScoreTimeSeries = computed(() => {
    if (!data.value?.volZScoreTimeSeries) return []
    return data.value.volZScoreTimeSeries.map(point => ({
      timestamp: point.timestamp * 1000, // Convert to milliseconds
      volZScore: point.volZScore,
      symbol: point.symbol
    }))
  })
  
  // Volume time series from volumeSummarySeries
  const volumeTimeSeries = computed(() => {
    if (!data.value?.volumeSummarySeries?.[0]?.series) return []
    // Extract the series array from the first item (assuming single coin)
    return data.value.volumeSummarySeries[0].series.map(point => ({
      timestamp: point.timestamp * 1000, // Convert to milliseconds
      dollarVolume: point.dollarVolume,
      ewmaDollarVolume: point.ewmaDollarVolume,
      avgDailyDollarVolume: point.avgDailyDollarVolume,
      weeklyDollarVolume: point.weeklyDollarVolume,
      avgHourlyDollarVolume: point.avgHourlyDollarVolume
    }))
  })
  
  // Get current values from volZScoreVsReturns (latest data)
  const currentVolatilitySummary = computed(() => {
    if (!data.value?.volZScoreVsReturns || data.value.volZScoreVsReturns.length === 0) return null
    // Get the last item which should be the most recent
    return data.value.volZScoreVsReturns[data.value.volZScoreVsReturns.length - 1]
  })
  
  const currentVolZScore = computed(() => {
    return currentVolatilitySummary.value?.volZScore || 0
  })
  
  const currentDailyRange = computed(() => {
    return currentVolatilitySummary.value?.dailyRange || 0
  })
  
  const currentVolume = computed(() => {
    return currentVolatilitySummary.value?.volume || 0
  })
  
  // Current volume summary
  const currentVolumeSummary = computed(() => {
    if (!data.value?.volumeSummaries?.[0]) return null
    return data.value.volumeSummaries[0]
  })
  
  // Volume metrics
  const avgDailyVolume = computed(() => {
    return currentVolumeSummary.value?.avgDailyDollarVolume || 0
  })
  
  const avgHourlyVolume = computed(() => {
    return currentVolumeSummary.value?.avgHourlyDollarVolume || 0
  })
  
  const weeklyVolume = computed(() => {
    return currentVolumeSummary.value?.weeklyDollarVolume || 0
  })
  
  const volumeRatio = computed(() => {
    return currentVolumeSummary.value?.ratioCurrentToAvgDaily || 0
  })
  
  const ewmaRatio = computed(() => {
    return currentVolumeSummary.value?.ratioEWMAToAvgDaily || 0
  })
  
  // Volume distribution from API
  const volumeDistribution = computed(() => {
    if (!data.value?.volumeDistribution) return null
    return {
      buckets: data.value.volumeDistribution.buckets.map(b => `${b}σ`),
      counts: data.value.volumeDistribution.counts,
      currentZScore: data.value.volumeDistribution.currentValue || 0
    }
  })
  

  const spreadTimeSeries = computed(() => {
  if (!data.value?.spreadSummarySeries?.[0]?.series) return []
  return data.value.spreadSummarySeries[0].series.map(point => ({
    timestamp: point.timestamp * 1000,
    spread1h: point.spread1h,
    avg1d: point.avg1d,
    std1d: point.std1d,
    avg7d: point.avg7d,
    std7d: point.std7d,
    avgZScore: point.avgZScore
  }))
  })

  

  const liquidityTimeSeries = computed(() => {
  const volumes = volumeTimeSeries.value
  const spreads = spreadTimeSeries.value
  
  if (!volumes.length) return []
  
  return volumes.map((point, index) => {
    // Find matching spread data by timestamp
    const spreadPoint = spreads.find(s => s.timestamp === point.timestamp)
    
    return {
      timestamp: point.timestamp,
      depth: point.ewmaDollarVolume,
      volume: point.dollarVolume,
      spread: spreadPoint ? spreadPoint.spread1h : calculateSpread(point.dollarVolume, point.avgDailyDollarVolume),
      avgSpread: spreadPoint?.avg1d || 0,
      spreadZScore: spreadPoint?.avgZScore || 0
    }
  })
  })
  

  const spreadVolumeScatterData = computed(() => {
    const volumes = volumeTimeSeries.value
    const spreads = spreadTimeSeries.value
    
    if (!volumes.length || !spreads.length) return []
    
    return volumes.map((vol, index) => {
      const spread = spreads.find(s => s.timestamp === vol.timestamp)
      if (!spread) return null
      
      return {
        timestamp: vol.timestamp,
        volume: vol.dollarVolume,
        ewmaVolume: vol.ewmaDollarVolume,
        spread: spread.avg7d * 100, // Use 7-day average spread instead of 1h
        avgSpread: spread.avg1d * 100,
        spreadZScore: spread.avgZScore,
        volumeRatio: vol.dollarVolume / vol.avgDailyDollarVolume,
        label: new Date(vol.timestamp).toLocaleString()
      }
    }).filter(Boolean) as any[]
  })

  // Volume ratio time series
  const volumeRatioTimeSeries = computed(() => {
    if (!volumeTimeSeries.value.length) return []
    return volumeTimeSeries.value.map(point => ({
      timestamp: point.timestamp,
      currentRatio: point.dollarVolume / point.avgDailyDollarVolume,
      ewmaRatio: point.ewmaDollarVolume / point.avgDailyDollarVolume
    }))
  })
  
  // Helper function to calculate spread based on volume deviation
  function calculateSpread(currentVolume: number, avgVolume: number): number {
    const ratio = currentVolume / avgVolume
    // Higher volume generally means tighter spreads
    if (ratio > 2) return 0.01  // Very high volume = tight spread
    if (ratio > 1.5) return 0.012
    if (ratio > 1) return 0.015
    if (ratio > 0.5) return 0.02
    return 0.03 // Low volume = wide spread
  }
  
  // Fetch data
  async function fetchData(params: {
    coin: string
    period: string
    exchange: string
    marketType: string
    timeframe: string 
  }) {
    loading.value = true
    error.value = null
    
    try {
      const response = await fetchVolatilityLiquidity(params) // Now `params` has timeframe
      data.value = response
      
      // Debug logging
      console.log('API Response:', response)
      console.log('Volume Summary Series:', response.volumeSummarySeries?.[0])
      console.log('Processed volume time series:', volumeTimeSeries.value.slice(0, 5))
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch volatility liquidity data'
      console.error('Volatility liquidity fetch error:', e)
    } finally {
      loading.value = false
    }
  }
  
  function clear() {
    data.value = null
    error.value = null
  }
  
  return {
    // State
    data,
    loading,
    error,
    
    // Time series data
    volZScoreTimeSeries,
    volumeTimeSeries,
    liquidityTimeSeries,
    volumeRatioTimeSeries,
    
    // Current values
    currentVolZScore,
    currentVolatilitySummary,
    currentVolumeSummary,
    currentDailyRange,
    currentVolume,
    avgDailyVolume,
    avgHourlyVolume,
    weeklyVolume,
    volumeRatio,
    ewmaRatio,
    volumeDistribution,
    
    spreadTimeSeries,
    spreadVolumeScatterData,
    fetchData,
    clear,
    hasData: computed(() => data.value !== null)
  }
}
