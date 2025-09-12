import { ref, computed } from 'vue'
import { fetchRelativeStrength } from '@/api/zscore'
import type { RelativeStrengthResponse } from '@/api/zscore'

export interface RSRanking {
  rank: number
  symbol: string
  pair: string
  rsZScore: number
  change24h: number
  volume: number
}

export interface PairDivergence {
  pair: string
  coin1: string
  coin2: string
  spread: number
  zScore: number
  historicalRange: { min: number; max: number }
  timeSeries: Array<{
    timestamp: number
    spread: number
    zScore: number
  }>
}

export interface MomentumPersistence {
  category: 'strong' | 'medium' | 'weak'
  correlation: number
  coins: string[]
  description: string
}

export interface MomentumFactor {
  symbol: string
  beta: number
  loading: number
  r2: number
  category: 'high' | 'medium' | 'low'
}

export function useRelativeStrength() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  // Request params
  const baseCoin = ref('BTC')
  const period = ref('7d')
  const exchange = ref('binance')
  const topN = ref(50)
  
  // Raw data from API
  const rawData = ref<RelativeStrengthResponse | null>(null)
  
  // Computed RS Rankings
  const rsRankings = computed<RSRanking[]>(() => {
    if (!rawData.value?.rsRankings) return []
    
    return rawData.value.rsRankings.map(ranking => {
      // Extract symbol from pair (e.g., "TRX/BTC" -> "TRX")
      const symbol = ranking.symbol.split('/')[0]
      
      // Get RS series data for additional metrics
      const rsSeries = rawData.value?.rsSeries?.[symbol]?.series || []
      const latestValue = rsSeries[rsSeries.length - 1]?.value || 0
      const previousValue = rsSeries[rsSeries.length - 2]?.value || 0
      const change24h = previousValue ? (latestValue - previousValue) / previousValue : 0
      
      return {
        rank: ranking.rank,
        symbol,
        pair: ranking.symbol,
        rsZScore: ranking.z || 0,
        change24h,
        volume: 0 // Volume would need to come from another endpoint
      }
    })
  })
  
  // Top and bottom performers
  const topPerformers = computed(() => rsRankings.value.slice(0, 5))
  const bottomPerformers = computed(() => rsRankings.value.slice(-5).reverse())
  
  // Pair divergence analysis - FIXED TYPE COMPATIBILITY
  const pairDivergences = computed<PairDivergence[]>(() => {
    if (!rawData.value?.pairDivergence) return []
    
    return rawData.value.pairDivergence
      .filter(div => div.timeSeries && div.timeSeries.length > 0)
      .map(div => {
        const timeSeries = div.timeSeries || []
        
        // Filter out undefined spreads and ensure they are numbers
        const validSpreads = timeSeries
          .map(t => t.spread)
          .filter((spread): spread is number => spread !== undefined && spread !== null)
        
        const validZScores = timeSeries
          .map(t => t.zscore)
          .filter((zscore): zscore is number => zscore !== undefined && zscore !== null)
        
        // Use the last valid values or defaults
        const lastSpread = validSpreads.length > 0 ? validSpreads[validSpreads.length - 1] : 0
        const lastZScore = validZScores.length > 0 ? validZScores[validZScores.length - 1] : 0
        
        // Calculate historical range only from valid numbers
        const historicalMin = validSpreads.length > 0 ? Math.min(...validSpreads) : 0
        const historicalMax = validSpreads.length > 0 ? Math.max(...validSpreads) : 0
        
        // Create time series with guaranteed number types
        const processedTimeSeries = timeSeries.map(t => ({
          timestamp: t.timestamp,
          spread: t.spread !== undefined && t.spread !== null ? t.spread : 0,
          zScore: t.zscore !== undefined && t.zscore !== null ? t.zscore : Math.abs(t.spread || 0)
        }))
        
        return {
          pair: div.pair,
          coin1: div.pair.split('-')[0],
          coin2: div.pair.split('-')[1],
          spread: lastSpread,
          zScore: lastZScore,
          historicalRange: {
            min: historicalMin,
            max: historicalMax
          },
          timeSeries: processedTimeSeries
        }
      })
      .filter(d => d.zScore > 1.5) // Only extreme divergences
  })
  
  // Momentum persistence analysis - FIXED TYPE COMPATIBILITY
  const momentumPersistence = computed(() => {
    if (!rawData.value?.persistence) return []
    
    // Group by persistence strength
    const grouped = rawData.value.persistence.reduce((acc, item) => {
      let category: 'strong' | 'medium' | 'weak'
      if (item.rho1 > 0.7) category = 'strong'
      else if (item.rho1 > 0.3) category = 'medium'
      else category = 'weak'
      
      if (!acc[category]) acc[category] = []
      acc[category].push(item.symbol)
      return acc
    }, {} as Record<string, string[]>)
    
    const result = [
      {
        category: 'strong' as const,
        correlation: 0.85, // Average of strong correlations
        coins: grouped.strong || [],
        description: 'ρ > 0.7'
      },
      {
        category: 'medium' as const,
        correlation: 0.5,
        coins: grouped.medium || [],
        description: '0.3 - 0.7'
      },
      {
        category: 'weak' as const,
        correlation: 0.15,
        coins: grouped.weak || [],
        description: 'ρ < 0.3'
      }
    ].filter(cat => cat.coins.length > 0)
    
    return result
  })
  
  // Cross-sectional momentum factors
  const momentumFactors = computed<MomentumFactor[]>(() => {
    if (!rawData.value?.momentumFactorLoadings) return []
    
    return rawData.value.momentumFactorLoadings
      .map(factor => {
        const category: 'high' | 'medium' | 'low' = 
          factor.beta > 1.5 ? 'high' : 
          factor.beta > 0.8 ? 'medium' : 'low'
        
        return {
          symbol: factor.symbol,
          beta: factor.beta,
          loading: factor.beta * (factor.r2 || 1), // Weight by R²
          r2: factor.r2,
          category
        }
      })
      .filter(f => f.category === 'high' || (f.category === 'medium' && f.r2 > 0.5))
      .sort((a, b) => b.loading - a.loading)
      .slice(0, 10)
  })
  
  const fetchData = async () => {
    loading.value = true
    error.value = null
    
    try {
      const data = await fetchRelativeStrength({
        exchange: exchange.value,
        base: baseCoin.value,
        period: period.value,
        topN: topN.value,
        marketType: 'spot'
      })
      
      rawData.value = data
      
      // Update base coin if different from response
      if (data.base) {
        baseCoin.value = data.base
      }
      
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch relative strength data'
      console.error('Relative strength fetch error:', e)
    } finally {
      loading.value = false
    }
  }
  
  return {
    // State
    loading,
    error,
    
    // Params
    baseCoin,
    period,
    exchange,
    topN,
    
    // Data
    rsRankings,
    topPerformers,
    bottomPerformers,
    pairDivergences,
    momentumPersistence,
    momentumFactors,
    
    // Methods
    fetchData
  }
}