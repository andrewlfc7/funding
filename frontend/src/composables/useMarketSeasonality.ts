// src/composables/useMarketSeasonality.ts
import { ref, computed } from 'vue'
import { 
  fetchMarketSeasonality, 
  type MarketSeasonalityResponse,
  type MarketSeasonalityParams,
  type SeasonalityAnomaly,
  type VolumePersistence,
  type ActivityPeriod,
  type Pattern,
  type VolatilityCluster,
  type WeekendEffect,
  type TimezoneEffect
} from '@/api/zscore/marketSeasonality'

export function useMarketSeasonality() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  const data = ref<MarketSeasonalityResponse | null>(null)

  // Computed properties for easy access
  const intradayHeatmap = computed(() => {
    if (!data.value?.intradayHeatmap) return { labels: [], data: [] }
    
    // Create labels for hours (0-23)
    const hourLabels = Array.from({ length: 24 }, (_, i) => `${i}:00`)
    
    // Get asset labels from the data
    const assetLabels = data.value.intradayHeatmap.map((_, index) => `Asset ${index + 1}`)
    
    return {
      xLabels: hourLabels,
      yLabels: assetLabels,
      data: data.value.intradayHeatmap
    }
  })

  const weekdayVolatilityHeatmap = computed(() => {
    if (!data.value?.weekdayHeatmap?.volatility) return { labels: [], data: [] }
    
    const dayLabels = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
    const assetLabels = data.value.weekdayHeatmap.volatility.map((_, index) => `Asset ${index + 1}`)
    
    return {
      xLabels: dayLabels,
      yLabels: assetLabels,
      data: data.value.weekdayHeatmap.volatility
    }
  })

  const weekdayVolumeHeatmap = computed(() => {
    if (!data.value?.weekdayHeatmap?.volume) return { labels: [], data: [] }
    
    const dayLabels = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
    const assetLabels = data.value.weekdayHeatmap.volume.map((_, index) => `Asset ${index + 1}`)
    
    return {
      xLabels: dayLabels,
      yLabels: assetLabels,
      data: data.value.weekdayHeatmap.volume
    }
  })

  const monthlySeasonality = computed(() => {
    if (!data.value?.monthlySeasonality) return []
    
    return Object.entries(data.value.monthlySeasonality).map(([asset, values]) => ({
      asset,
      returns: values.returns,
      volatility: values.volatility
    }))
  })

  const volumeAutocorrelation = computed(() => {
    if (!data.value?.volumeAutocorrelation) return []
    
    return Object.entries(data.value.volumeAutocorrelation).map(([asset, lags]) => ({
      asset,
      lags
    }))
  })

  const topAnomalies = computed(() => {
    if (!data.value?.anomalies) return []
    
    return data.value.anomalies
      .sort((a, b) => Math.abs(b.zscore) - Math.abs(a.zscore))
      .slice(0, 12)
  })

  const volumePersistence = computed(() => {
    if (!data.value?.volumePersistence) return []
    
    return data.value.volumePersistence
      .sort((a, b) => b.lag1h - a.lag1h)
  })

  const mostActivePeriods = computed(() => data.value?.mostActivePeriods || [])
  const leastActivePeriods = computed(() => data.value?.leastActivePeriods || [])
  const strongestPatterns = computed(() => data.value?.strongestPatterns || [])
  const volatilityClusters = computed(() => data.value?.volatilityClusters || [])
  const weekendEffect = computed(() => data.value?.weekendEffect || [])
  const timezoneEffects = computed(() => data.value?.timezoneEffects || [])

  // Stats computed
  const stats = computed(() => {
    if (!data.value) {
      return {
        totalAnomalies: 0,
        averageWeekendEffect: 0,
        strongestPattern: null,
        mostPersistentAsset: null
      }
    }

    const avgWeekendEffect = data.value.weekendEffect.length > 0
      ? data.value.weekendEffect.reduce((sum, item) => sum + item.effect, 0) / data.value.weekendEffect.length
      : 0

    const strongestPattern = data.value.strongestPatterns
      .sort((a, b) => b.strength - a.strength)[0] || null

    const mostPersistentAsset = data.value.volumePersistence
      .sort((a, b) => b.lag24h - a.lag24h)[0] || null

    return {
      totalAnomalies: data.value.anomalies.length,
      averageWeekendEffect: avgWeekendEffect,
      strongestPattern,
      mostPersistentAsset
    }
  })

  async function load(params: MarketSeasonalityParams) {
    loading.value = true
    error.value = null

    try {
      data.value = await fetchMarketSeasonality(params)
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to fetch market seasonality data'
      console.error('Market seasonality error:', err)
      data.value = null
    } finally {
      loading.value = false
    }
  }

  return {
    loading,
    error,
    data,
    // Heatmaps
    intradayHeatmap,
    weekdayVolatilityHeatmap,
    weekdayVolumeHeatmap,
    // Data arrays
    monthlySeasonality,
    volumeAutocorrelation,
    topAnomalies,
    volumePersistence,
    mostActivePeriods,
    leastActivePeriods,
    strongestPatterns,
    volatilityClusters,
    weekendEffect,
    timezoneEffects,
    // Stats
    stats,
    // Methods
    load
  }
}