// src/composables/useXSecSignals.ts
import { ref, computed, reactive } from 'vue'
import { 
  fetchXSecSignals, 
  fetchPortfolioConstruction, 
  fetchSignalPerformance, 
  fetchTrendDirection,
  type TrendSignalsRequest,
  type XSecSignalsResponse,
  type PortfolioConstructionResponse,
  type SignalPerformanceData,
  type TrendDirectionData
} from '@/api/trend/trend'

export function useXSecSignals() {
  // State
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  // Data storage
  const signals = ref<XSecSignalsResponse | null>(null)
  const portfolioData = ref<PortfolioConstructionResponse | null>(null)
  const performanceData = ref<SignalPerformanceData | null>(null)
  const directionData = ref<TrendDirectionData | null>(null)
  
  // UI state
  const selectedSymbol = ref('BTC')
  
  // Computed properties
  const symbols = computed(() => signals.value?.symbols || [])
  
  const series = computed(() => {
    if (!signals.value) return {}
    return signals.value.series
  })
  
  const correlations = computed(() => {
    if (!signals.value) return {}
    return signals.value.correlations
  })
  
  const summary = computed(() => {
    if (!signals.value) return null
    return signals.value.summary
  })

  // Chart data adapters
  const chartLabels = computed(() => {
    const symbolData = series.value[selectedSymbol.value]
    if (!symbolData) return []
    return symbolData.map(point => new Date(point.timestamp * 1000))
  })

  const signalDataById = computed(() => {
    const symbolData = series.value[selectedSymbol.value]
    if (!symbolData) return {}
    
    return {
      momentum: symbolData.map(p => p.momentum),
      ewmac: symbolData.map(p => p.ewmac),
      breakout: symbolData.map(p => p.breakout),
      composite: symbolData.map(p => p.composite)
    }
  })

  const priceData = computed(() => {
    const symbolData = series.value[selectedSymbol.value]
    if (!symbolData) return []
    return symbolData.map(p => p.price)
  })

  const volumeData = computed(() => {
    const symbolData = series.value[selectedSymbol.value]
    if (!symbolData) return []
    return symbolData.map(p => p.volume)
  })

  const returnsData = computed(() => {
    const symbolData = series.value[selectedSymbol.value]
    if (!symbolData) return []
    return symbolData.map(p => p.returns)
  })

  const volatilityData = computed(() => {
    const symbolData = series.value[selectedSymbol.value]
    if (!symbolData) return []
    return symbolData.map(p => p.volatility)
  })

  // Portfolio exposure data for heatmap
  const exposurePositions = computed(() => {
    if (!portfolioData.value) return []
    
    return portfolioData.value.positions.map(pos => ({
      asset: pos.asset,
      weight: pos.weight,
      volatility: pos.volatility,
      signal: pos.signal,
      expected_return: pos.expected_return,
      risk_contribution: pos.contribution,
      pnl: pos.contribution // Using contribution as proxy for P&L
    }))
  })

  // Signal performance metrics
  const performanceMetrics = computed(() => {
    if (!performanceData.value) return null
    return performanceData.value.performance_metrics
  })

  // Load functions
  async function load(params: TrendSignalsRequest) {
    loading.value = true
    error.value = null
    
    try {
      const data = await fetchXSecSignals(params)
      signals.value = data
      
      // Auto-select first symbol if none selected
      if (data.symbols.length > 0 && !selectedSymbol.value) {
        selectedSymbol.value = data.symbols[0]
      }
      
    } catch (err: any) {
      error.value = err?.message || 'Failed to load signals'
      console.error('Error loading cross-sectional signals:', err)
    } finally {
      loading.value = false
    }
  }

  async function loadPortfolio(params: Parameters<typeof fetchPortfolioConstruction>[0]) {
    loading.value = true
    error.value = null
    
    try {
      portfolioData.value = await fetchPortfolioConstruction(params)
    } catch (err: any) {
      error.value = err?.message || 'Failed to load portfolio data'
      console.error('Error loading portfolio:', err)
    } finally {
      loading.value = false
    }
  }

  async function loadPerformance(params: Parameters<typeof fetchSignalPerformance>[0]) {
    loading.value = true
    error.value = null
    
    try {
      performanceData.value = await fetchSignalPerformance(params)
    } catch (err: any) {
      error.value = err?.message || 'Failed to load performance data'
      console.error('Error loading performance:', err)
    } finally {
      loading.value = false
    }
  }

  async function loadDirection(params: TrendSignalsRequest) {
    loading.value = true
    error.value = null
    
    try {
      directionData.value = await fetchTrendDirection(params)
    } catch (err: any) {
      error.value = err?.message || 'Failed to load direction data'
      console.error('Error loading direction:', err)
    } finally {
      loading.value = false
    }
  }

  // Utility functions
  function getSignalStrength(signalValue: number): 'weak' | 'medium' | 'strong' {
    const abs = Math.abs(signalValue)
    if (abs > 2) return 'strong'
    if (abs > 1) return 'medium'
    return 'weak'
  }

  function getSignalColor(signalType: string): string {
    const colors = {
      momentum: '#00BF63',
      ewmac: '#00D4FF', 
      breakout: '#FF6B6B',
      composite: '#FFA502'
    }
    return colors[signalType as keyof typeof colors] || '#ECF0F1'
  }

  function formatSignalValue(value: number): string {
    return value.toFixed(2)
  }

  function getCurrentSignalValues() {
    const symbolData = series.value[selectedSymbol.value]
    if (!symbolData || symbolData.length === 0) return null
    
    const latest = symbolData[symbolData.length - 1]
    return {
      momentum: latest.momentum,
      ewmac: latest.ewmac,
      breakout: latest.breakout,
      composite: latest.composite
    }
  }

  // Reset function
  function reset() {
    signals.value = null
    portfolioData.value = null
    performanceData.value = null
    directionData.value = null
    error.value = null
    loading.value = false
  }

  return {
    // State
    loading,
    error,
    selectedSymbol,
    
    // Data
    signals,
    portfolioData,
    performanceData,
    directionData,
    symbols,
    series,
    correlations,
    summary,
    
    // Chart data
    chartLabels,
    signalDataById,
    priceData,
    volumeData,
    returnsData,
    volatilityData,
    exposurePositions,
    performanceMetrics,
    
    // Actions
    load,
    loadPortfolio,
    loadPerformance, 
    loadDirection,
    reset,
    
    // Utilities
    getSignalStrength,
    getSignalColor,
    formatSignalValue,
    getCurrentSignalValues
  }
}