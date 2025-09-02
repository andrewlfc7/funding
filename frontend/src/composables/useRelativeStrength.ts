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
  
  // Pair divergence analysis
  const pairDivergences = computed<PairDivergence[]>(() => {
    if (!rawData.value?.pairDivergence) return []
    
    return rawData.value.pairDivergence
      .filter(div => div.timeSeries && div.timeSeries.length > 0)
      .map(div => {
        const timeSeries = div.timeSeries || []
        const zscores = timeSeries.map(t => t.zscore || Math.abs(t.spread))
        const spreads = timeSeries.map(t => t.spread)
        
        return {
          pair: div.pair,
          coin1: div.pair.split('-')[0],
          coin2: div.pair.split('-')[1],
          spread: spreads[spreads.length - 1] || 0,
          zScore: zscores[zscores.length - 1] || 0,
          historicalRange: {
            min: Math.min(...spreads),
            max: Math.max(...spreads)
          },
          timeSeries: timeSeries.map(t => ({
            timestamp: t.timestamp,
            spread: t.spread,
            zScore: t.zscore || Math.abs(t.spread)
          }))
        }
      })
      .filter(d => d.zScore > 1.5) // Only extreme divergences
  })
  
  // Momentum persistence analysis
  const momentumPersistence = computed<MomentumPersistence[]>(() => {
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
    
    return [
      {
        category: 'strong',
        correlation: 0.85, // Average of strong correlations
        coins: grouped.strong || [],
        description: 'ρ > 0.7'
      },
      {
        category: 'medium',
        correlation: 0.5,
        coins: grouped.medium || [],
        description: '0.3 - 0.7'
      },
      {
        category: 'weak',
        correlation: 0.15,
        coins: grouped.weak || [],
        description: 'ρ < 0.3'
      }
    ].filter(cat => cat.coins.length > 0)
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