// src/composables/useTradesAnalysis.ts
import { ref, computed } from 'vue'
import { API_BASE_URL } from '@/utils/constants'

interface TradeImbalance {
  symbol: string
  buyRatio: number
  volume: number
  avgTradeSize: number
  imbalance: number
}

interface WhaleTrade {
  id: string
  symbol: string
  timestamp: number
  size: number
  price: number
  side: 'buy' | 'sell'
}

interface TradeZScore {
  name: string
  type: 'count' | 'size' | 'percent'
  current: number
  average: number
  zscore: number
}

interface MicrostructureStat {
  name: string
  value: string
}

export function useTradesAnalysis() {
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Data refs
  const imbalanceScatterData = ref<TradeImbalance[]>([])
  const sizeDistributionData = ref<any>(null)
  const notionalRankingsData = ref<any[]>([])
  const imbalanceTimeSeriesData = ref<any[]>([])
  const velocityHeatmapData = ref<number[][]>([])
  const whaleTradesData = ref<WhaleTrade[]>([])
  const correlationMatrixData = ref<number[][]>([])
  const tradeZScoresData = ref<TradeZScore[]>([])
  const microstructureStatsData = ref<MicrostructureStat[]>([])

  // Computed properties
  const imbalanceScatter = computed(() => imbalanceScatterData.value)
  const sizeDistribution = computed(() => sizeDistributionData.value)
  const notionalRankings = computed(() => notionalRankingsData.value)
  const imbalanceTimeSeries = computed(() => imbalanceTimeSeriesData.value)
  const velocityHeatmap = computed(() => velocityHeatmapData.value)
  const whaleTrades = computed(() => whaleTradesData.value.slice(0, 10))
  const correlationMatrix = computed(() => correlationMatrixData.value)
  const tradeZScores = computed(() => tradeZScoresData.value)
  const microstructureStats = computed(() => microstructureStatsData.value)

  async function fetchData(params: {
    assets: string[]
    interval: string
    period: string
    distributionAsset?: string
  }) {
    loading.value = true
    error.value = null

    try {
      const response = await fetch(
        `${API_BASE_URL}/zscore/trades-analysis?` + new URLSearchParams({
          assets: params.assets.join(','),
          interval: params.interval,
          period: params.period,
          ...(params.distributionAsset && { distributionAsset: params.distributionAsset })
        })
      )

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`)
      }

      const data = await response.json()

      // Update all data refs
      imbalanceScatterData.value = data.imbalanceScatter || []
      sizeDistributionData.value = data.sizeDistribution || null
      notionalRankingsData.value = data.notionalRankings || []
      imbalanceTimeSeriesData.value = data.imbalanceTimeSeries || []
      velocityHeatmapData.value = data.velocityHeatmap || []
      whaleTradesData.value = data.whaleTrades || []
      correlationMatrixData.value = data.correlationMatrix || []
      tradeZScoresData.value = data.tradeZScores || []
      microstructureStatsData.value = data.microstructureStats || []

    } catch (err) {
      error.value = err instanceof Error ? err.message : 'Failed to fetch data'
      console.error('Error fetching trades analysis:', err)
    } finally {
      loading.value = false
    }
  }

  return {
    loading,
    error,
    imbalanceScatter,
    sizeDistribution,
    notionalRankings,
    imbalanceTimeSeries,
    velocityHeatmap,
    whaleTrades,
    correlationMatrix,
    tradeZScores,
    microstructureStats,
    fetchData
  }
}