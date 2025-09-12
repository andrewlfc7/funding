// frontend/src/composables/useMarketMicrostructure.ts
import { ref, computed } from 'vue'
import { getMarketMicrostructureFlow } from '@/api/zscore/marketMicroStructure'
import { fetchMetaData } from '@/api/meta'

export interface VolumeFlowData {
  symbol: string
  volumeIn: number
  volumeOut: number
  netFlow: number
  netVolumeZScore: number // Changed from netFlowZScore to match Vue component
}

export interface RotationMatrixData {
  from: string
  to: string
  flow: number
  percentage: number
}

export interface LiquidityConcentration {
  range: string
  volumeShare: number
  countShare: number
  percentage: number // Added to match Vue component usage
}

export function useMarketMicrostructure() {
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Request params
  const timeframe = ref('1h') // Only 1h is supported by backend
  const topN = ref(50)
  const period = ref('7d')
  const exchange = ref('binance')
  const marketType = ref('spot')

  // Raw data from API
  const microstructureData = ref<any>(null)
  const metaData = ref<{ coins: string[]; exchanges: string[] } | null>(null)

  // Computed data for volume flow
  const volumeFlow = computed<VolumeFlowData[]>(() => {
    if (!microstructureData.value?.volumeFlows) return []
    
    return microstructureData.value.volumeFlows
      .map((flow: any) => ({
        symbol: flow.symbol,
        volumeIn: flow.volumeIn,
        volumeOut: flow.volumeOut,
        netFlow: flow.netFlow,
        netVolumeZScore: flow.netFlowZScore // Map netFlowZScore to netVolumeZScore
      }))
      .sort((a: VolumeFlowData, b: VolumeFlowData) => 
        Math.abs(b.netVolumeZScore) - Math.abs(a.netVolumeZScore)
      )
  })

  // Computed rotation matrix
  const rotationMatrix = computed<RotationMatrixData[]>(() => {
    if (!microstructureData.value?.rotationMatrix) return []
    
    const { coins, flows } = microstructureData.value.rotationMatrix
    const matrixData: RotationMatrixData[] = []
    
    for (let i = 0; i < coins.length; i++) {
      for (let j = 0; j < coins.length; j++) {
        if (i !== j && flows[i] && flows[i][j] > 0) {
          matrixData.push({
            from: coins[i],
            to: coins[j],
            flow: flows[i][j],
            percentage: (flows[i][j] / Math.max(...flows.flat())) * 100
          })
        }
      }
    }
    
    return matrixData.sort((a, b) => b.flow - a.flow)
  })

  // Computed liquidity concentration
  const liquidityConcentration = computed<LiquidityConcentration[]>(() => {
    if (!microstructureData.value?.liquidityConcentration?.groups) return []
    
    return microstructureData.value.liquidityConcentration.groups
      .map((group: any) => ({
        range: group.range,
        volumeShare: group.volumeShare * 100, // Convert to percentage
        countShare: group.countShare * 100,   // Convert to percentage
        percentage: group.volumeShare * 100   // Add percentage field for Vue component
      }))
  })

  // Get top coins for display
  const topCoins = computed(() => {
    if (!microstructureData.value?.rotationMatrix?.coins) return []
    return microstructureData.value.rotationMatrix.coins
  })

  const fetchData = async () => {
    loading.value = true
    error.value = null

    try {
      // Fetch meta data first
      const metaResponse = await fetchMetaData(marketType.value, 'USDT')
      metaData.value = metaResponse

      // Fetch microstructure data
      const data = await getMarketMicrostructureFlow({
        exchange: exchange.value,
        timeframe: timeframe.value,
        period: period.value,
        marketType: marketType.value,
        topN: topN.value
      })

      microstructureData.value = data
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch market microstructure data'
      console.error('Market microstructure fetch error:', e)
    } finally {
      loading.value = false
    }
  }

  return {
    // State
    loading,
    error,

    // Params
    timeframe,
    topN,
    period,
    exchange,
    marketType,

    // Meta data
    metaData,
    topCoins,

    // Computed data
    volumeFlow,
    rotationMatrix,
    liquidityConcentration,

    // Methods
    fetchData
  }
}