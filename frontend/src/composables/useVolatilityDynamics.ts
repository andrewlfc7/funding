// src/composables/useVolatilityDynamics.ts
import { ref, computed } from 'vue'
import { 
  fetchVolatilityDynamics,
  type VolatilityDynamicsResponse,
  type VolatilityDynamicsParams,
  type TimeSeriesData,
  type SkewKurtosisData,
  type RegimeData,
  type InstabilityData,
  type FlowVolBeta,
  type VolStats,
  type VolDistribution,
  type DistributionStat
} from '@/api/zscore/volatilityDynamics'

export function useVolatilityDynamics() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  const data = ref<VolatilityDynamicsResponse | null>(null)

  // Computed properties - ensure all API data is properly exposed
  const vovTimeSeries = computed(() => data.value?.vovTimeSeries || [])
  const skewnessTimeSeries = computed(() => data.value?.skewnessTimeSeries || [])
  const skewKurtosisScatter = computed(() => data.value?.skewKurtosisScatter || [])
  const volDistribution = computed(() => data.value?.volDistribution || null)
  const distributionStats = computed(() => data.value?.distributionStats || [])
  const covarianceTimeSeries = computed(() => data.value?.covarianceTimeSeries || [])
  const regimeClassification = computed(() => data.value?.regimeClassification || [])
  const instabilityRankings = computed(() => data.value?.instabilityRankings || [])
  const flowVolBeta = computed(() => data.value?.flowVolBeta || [])
  const volStatsSummary = computed(() => data.value?.volStatsSummary || [])
  
  // Enhanced computed properties with sorting and filtering
  const topInstabilityRankings = computed(() => {
    return [...instabilityRankings.value]
      .sort((a, b) => b.vov - a.vov)
      .slice(0, 10)
  })
  
  const topFlowVolBeta = computed(() => {
    return [...flowVolBeta.value]
      .sort((a, b) => Math.abs(b.beta) - Math.abs(a.beta)) // Sort by absolute beta for most impactful
      .slice(0, 10)
  })
  
  const regimeCounts = computed(() => {
    const counts = {
      normal: 0,
      stressed: 0,
      euphoric: 0,
      compressed: 0,
      unstable: 0
    }
    
    regimeClassification.value.forEach(item => {
      if (counts.hasOwnProperty(item.regime)) {
        counts[item.regime as keyof typeof counts]++
      }
    })
    
    return counts
  })

  // Enhanced stats computed with more comprehensive metrics
  const stats = computed(() => {
    if (!data.value) {
      return {
        avgVov: 0,
        avgSkewness: 0,
        avgKurtosis: 0,
        avgBeta: 0,
        criticalCount: 0,
        warningCount: 0,
        normalCount: 0,
        totalAssets: 0,
        regimeBreakdown: regimeCounts.value
      }
    }

    const vovValues = volStatsSummary.value.map(s => s.vov).filter(v => !isNaN(v))
    const skewValues = volStatsSummary.value.map(s => s.skewness).filter(v => !isNaN(v))
    const kurtosisValues = volStatsSummary.value.map(s => s.kurtosis).filter(v => !isNaN(v))
    const betaValues = flowVolBeta.value.map(s => s.beta).filter(v => !isNaN(v))

    const critical = instabilityRankings.value.filter(i => i.status === 'critical').length
    const warning = instabilityRankings.value.filter(i => i.status === 'warning').length
    const normal = instabilityRankings.value.filter(i => i.status === 'normal').length

    return {
      avgVov: vovValues.length ? vovValues.reduce((a, b) => a + b, 0) / vovValues.length : 0,
      avgSkewness: skewValues.length ? skewValues.reduce((a, b) => a + b, 0) / skewValues.length : 0,
      avgKurtosis: kurtosisValues.length ? kurtosisValues.reduce((a, b) => a + b, 0) / kurtosisValues.length : 0,
      avgBeta: betaValues.length ? betaValues.reduce((a, b) => a + b, 0) / betaValues.length : 0,
      criticalCount: critical,
      warningCount: warning,
      normalCount: normal,
      totalAssets: volStatsSummary.value.length,
      regimeBreakdown: regimeCounts.value
    }
  })

  // Helper function to get time series data for a specific symbol
  const getTimeSeriesForSymbol = computed(() => (symbol: string, type: 'vov' | 'skewness' | 'covariance') => {
    let series: TimeSeriesData[] = []
    
    switch (type) {
      case 'vov':
        series = vovTimeSeries.value
        break
      case 'skewness':
        series = skewnessTimeSeries.value
        break
      case 'covariance':
        series = covarianceTimeSeries.value
        break
    }
    
    return series.find(s => s.symbol === symbol)?.data || []
  })

  // Helper function to get regime color
  const getRegimeColor = (regime: string): string => {
    const colors = {
      normal: '#10B981',    // green
      stressed: '#EF4444',  // red
      euphoric: '#F59E0B',  // yellow
      compressed: '#3B82F6', // blue
      unstable: '#8B5CF6'   // purple
    }
    return colors[regime as keyof typeof colors] || '#6B7280' // gray as fallback
  }

  // Helper function to get status color
  const getStatusColor = (status: string): string => {
    const colors = {
      critical: '#EF4444',  // red
      warning: '#F59E0B',   // yellow
      normal: '#10B981'     // green
    }
    return colors[status as keyof typeof colors] || '#6B7280' // gray as fallback
  }

  async function load(params: {
    topN?: number
    timeframe?: string
    period: string
    distributionAsset?: string
    exchange?: string
    marketType?: string
  }) {
    loading.value = true
    error.value = null

    try {
      const apiParams: VolatilityDynamicsParams = {
        exchange: params.exchange,
        marketType: params.marketType,
        period: params.period,
        timeframe: params.timeframe,
        topN: params.topN || 20,
        histogramCoin: params.distributionAsset
      }

      data.value = await fetchVolatilityDynamics(apiParams)
      
      // Log for debugging
      console.log('Volatility dynamics data loaded:', {
        vovSeries: vovTimeSeries.value.length,
        skewSeries: skewnessTimeSeries.value.length,
        scatterPoints: skewKurtosisScatter.value.length,
        regimes: regimeClassification.value.length,
        instability: instabilityRankings.value.length,
        flowBeta: flowVolBeta.value.length,
        stats: volStatsSummary.value.length
      })
      
    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to fetch volatility dynamics data'
      console.error('Volatility dynamics error:', err)
      data.value = null
    } finally {
      loading.value = false
    }
  }

  // Updated fetchData to accept topN and other params
  async function fetchData(params: {
    topN: number
    timeframe: string
    period: string
    distributionAsset?: string
    exchange?: string
    marketType?: string
  }) {
    return load(params)
  }

  // Clear data function
  function clearData() {
    data.value = null
    error.value = null
  }

  return {
    // State
    loading,
    error,
    data,
    
    // Computed data
    vovTimeSeries,
    skewnessTimeSeries,
    skewKurtosisScatter,
    volDistribution,
    distributionStats,
    covarianceTimeSeries,
    regimeClassification,
    instabilityRankings: topInstabilityRankings,
    flowVolBeta: topFlowVolBeta,
    volStatsSummary,
    stats,
    regimeCounts,
    
    // Helper functions
    getTimeSeriesForSymbol,
    getRegimeColor,
    getStatusColor,
    
    // Methods
    load,
    fetchData,
    clearData,
    
    // Additional computed for UI convenience
    hasData: computed(() => data.value !== null),
    isEmpty: computed(() => {
      if (!data.value) return true
      return (
        vovTimeSeries.value.length === 0 &&
        skewnessTimeSeries.value.length === 0 &&
        volStatsSummary.value.length === 0
      )
    })
  }
}