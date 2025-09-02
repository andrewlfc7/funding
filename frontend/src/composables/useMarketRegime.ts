import { ref, computed } from 'vue'
import { fetchMarketRegime } from '@/api/zscore'
import type { MarketRegimeResponse } from '@/api/zscore'

export type MarketRegime = 'bull' | 'bear' | 'range'
export type TimeFrame = '1h' | '4h' | '1d' | '3d' | '7d'

export interface RegimeTransition {
  from: MarketRegime
  to: MarketRegime
  probability: number
}

export interface ZScoreMomentum {
  symbol: string
  timeframes: Record<string, number>
  velocity: number
  acceleration: number
}

export interface MomentumDivergence {
  category: 'leaders' | 'laggards'
  coins: string[]
  avgZScore: number
  divergenceScore: number
}

export function useMarketRegime() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  // Request params
  const period = ref('90d')
  const exchange = ref('binance')
  const topN = ref(10)
  
  // Raw data from API
  const rawData = ref<MarketRegimeResponse | null>(null)
  
  // Current market regime (derived from transition matrix)
  const currentRegime = ref<MarketRegime>('range')
  const nextRegimeProbability = ref<{ regime: MarketRegime; probability: number }>({
    regime: 'bull',
    probability: 0
  })
  
  // Z-Score momentum heatmap data
  const momentumHeatmap = computed<ZScoreMomentum[]>(() => {
    if (!rawData.value?.heatmap) return []
    
    const { coins, timeframes, matrix } = rawData.value.heatmap
    
    return coins.map((symbol, coinIndex) => {
      const timeframeData: Record<string, number> = {}
      let totalVelocity = 0
      
      timeframes.forEach((tf, tfIndex) => {
        const value = matrix[coinIndex]?.[tfIndex] || 0
        timeframeData[tf] = value
        totalVelocity += value
      })
      
      const velocity = totalVelocity / timeframes.length
      
      // Get acceleration from velocitySeries if available
      const velocitySeries = rawData.value?.velocitySeries?.find(v => v.symbol === symbol)
      const acceleration = velocitySeries?.series?.length > 1 
        ? velocitySeries.series[velocitySeries.series.length - 1].velocity - velocitySeries.series[0].velocity
        : 0
      
      return { symbol, timeframes: timeframeData, velocity, acceleration }
    })
  })
  
  // Regime transition matrix
  const transitionMatrix = computed<RegimeTransition[]>(() => {
    if (!rawData.value?.transitionMatrix) return []
    
    const regimes: MarketRegime[] = ['bull', 'bear', 'range']
    const transitions: RegimeTransition[] = []
    
    rawData.value.transitionMatrix.forEach((row, fromIndex) => {
      row.forEach((probability, toIndex) => {
        transitions.push({
          from: regimes[fromIndex],
          to: regimes[toIndex],
          probability
        })
      })
    })
    
    return transitions
  })
  
  // Z-Score velocity data for chart
  const velocityData = computed(() => {
    if (!rawData.value?.velocitySeries) return []
    
    // Aggregate velocity data across all symbols
    const aggregated = new Map<number, { velocity: number; count: number }>()
    
    rawData.value.velocitySeries.forEach(({ series }) => {
      series?.forEach(point => {
        const existing = aggregated.get(point.timestamp) || { velocity: 0, count: 0 }
        existing.velocity += point.velocity
        existing.count += 1
        aggregated.set(point.timestamp, existing)
      })
    })
    
    return Array.from(aggregated.entries())
      .map(([timestamp, data]) => ({
        timestamp: timestamp * 1000,
        velocity: data.velocity / data.count,
        acceleration: 0
      }))
      .sort((a, b) => a.timestamp - b.timestamp)
      .map((point, index, array) => {
        if (index > 0) {
          point.acceleration = point.velocity - array[index - 1].velocity
        }
        return point
      })
  })
  
  // Cross-asset momentum divergence
  const momentumDivergence = computed<MomentumDivergence[]>(() => {
    if (!rawData.value?.crossAssetDivergence) return []
    
    const { leaders, laggards, score } = rawData.value.crossAssetDivergence
    
    // Calculate average z-scores for leaders and laggards
    const leaderZScores = momentumHeatmap.value
      .filter(m => leaders.includes(m.symbol))
      .map(m => m.velocity)
    const laggardZScores = momentumHeatmap.value
      .filter(m => laggards.includes(m.symbol))
      .map(m => m.velocity)
    
    const leaderAvg = leaderZScores.length > 0
      ? leaderZScores.reduce((sum, v) => sum + v, 0) / leaderZScores.length
      : 0
    const laggardAvg = laggardZScores.length > 0
      ? laggardZScores.reduce((sum, v) => sum + v, 0) / laggardZScores.length
      : 0
    
    return [
      {
        category: 'leaders',
        coins: leaders,
        avgZScore: leaderAvg,
        divergenceScore: score
      },
      {
        category: 'laggards',
        coins: laggards,
        avgZScore: laggardAvg,
        divergenceScore: score
      }
    ]
  })
  
  // Determine current regime from transition matrix
  const determineCurrentRegime = () => {
    if (!rawData.value?.transitionMatrix) return
    
    // Find which regime has highest self-transition probability
    const regimes: MarketRegime[] = ['bull', 'bear', 'range']
    let maxProb = 0
    let regime: MarketRegime = 'range'
    
    rawData.value.transitionMatrix.forEach((row, index) => {
      if (row[index] > maxProb) {
        maxProb = row[index]
        regime = regimes[index]
      }
    })
    
    currentRegime.value = regime
    
    // Find most likely next regime
    const currentIndex = regimes.indexOf(regime)
    const transitions = rawData.value.transitionMatrix[currentIndex]
    let nextProb = 0
    let nextRegime: MarketRegime = regime
    
    transitions.forEach((prob, index) => {
      if (index !== currentIndex && prob > nextProb) {
        nextProb = prob
        nextRegime = regimes[index]
      }
    })
    
    if (nextProb > 0.1) { // Only show if probability is significant
      nextRegimeProbability.value = {
        regime: nextRegime,
        probability: nextProb
      }
    }
  }
  
  const fetchData = async () => {
    loading.value = true
    error.value = null
    
    try {
      const data = await fetchMarketRegime({
        exchange: exchange.value,
        period: period.value,
        topN: topN.value,
        marketType: 'spot'
      })
      
      rawData.value = data
      determineCurrentRegime()
      
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch market regime data'
      console.error('Market regime fetch error:', e)
    } finally {
      loading.value = false
    }
  }
  
  return {
    // State
    loading,
    error,
    
    // Params
    period,
    exchange,
    topN,
    
    // Data
    currentRegime,
    nextRegimeProbability,
    momentumHeatmap,
    transitionMatrix,
    velocityData,
    momentumDivergence,
    
    // Methods
    fetchData
  }
}