import { ref, computed } from 'vue'
import { fetchZScoreOverview, fetchVolatilityLiquidity } from '@/api/zscore'
import type { ZScoreDataPoint } from '@/api/zscore'

export interface VolumeFlowData {
  symbol: string
  netVolumeZScore: number
  volumeIn: number
  volumeOut: number
  netFlow: number
}

export interface RotationMatrixData {
  from: string
  to: string
  flow: number
  percentage: number
}

export interface LiquidityConcentration {
  range: string
  coins: string[]
  percentage: number
  volume: number
}

export interface HourlySeasonality {
  hour: number
  avgZScore: number
  volume: number
  count: number
}

export interface DayOfWeekEffect {
  day: string
  dayNum: number
  avgZScore: number
  avgReturns: number
  volume: number
}

export function useMarketMicrostructure() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  // Request params
  const timeframe = ref<'5min' | '15min' | '1h'>('5min')
  const topN = ref(50)
  const period = ref('7d')
  const exchange = ref('binance')
  
  // Raw data
  const rawData = ref<ZScoreDataPoint[]>([])
  const volumeData = ref<any>(null)
  
  // Computed data for volume flow
  const volumeFlow = computed<VolumeFlowData[]>(() => {
    if (!rawData.value.length) return []
    
    // Group by symbol and calculate net flows
    const symbolMap = new Map<string, VolumeFlowData>()
    
    rawData.value.forEach(point => {
      const existing = symbolMap.get(point.symbol) || {
        symbol: point.symbol,
        netVolumeZScore: 0,
        volumeIn: 0,
        volumeOut: 0,
        netFlow: 0
      }
      
      // Calculate based on z-score and volume
      const flow = point.zscore * (point.volume || 0)
      if (flow > 0) {
        existing.volumeIn += point.volume || 0
      } else {
        existing.volumeOut += Math.abs(point.volume || 0)
      }
      
      existing.netFlow = existing.volumeIn - existing.volumeOut
      existing.netVolumeZScore = point.zscore
      
      symbolMap.set(point.symbol, existing)
    })
    
    return Array.from(symbolMap.values())
      .sort((a, b) => Math.abs(b.netVolumeZScore) - Math.abs(a.netVolumeZScore))
      .slice(0, 10) // Top 10 for display
  })
  
  // Computed rotation matrix
  const rotationMatrix = computed<RotationMatrixData[]>(() => {
    if (!rawData.value.length) return []
    
    // Simplified rotation calculation - in real implementation, 
    // this would come from correlation/flow analysis
    const mainCoins = ['BTC', 'ETH', 'SOL', 'USDT']
    const matrix: RotationMatrixData[] = []
    
    mainCoins.forEach(from => {
      mainCoins.forEach(to => {
        if (from !== to) {
          // Mock calculation - replace with real flow data
          const fromData = rawData.value.find(d => d.symbol === from)
          const toData = rawData.value.find(d => d.symbol === to)
          
          if (fromData && toData) {
            const flow = (toData.zscore - fromData.zscore) * 1000000 // Mock flow
            matrix.push({
              from,
              to,
              flow,
              percentage: Math.abs(flow) / 10000000 * 100 // Mock percentage
            })
          }
        }
      })
    })
    
    return matrix
  })
  
  // Computed liquidity concentration
  const liquidityConcentration = computed<LiquidityConcentration[]>(() => {
    if (!volumeData.value?.volumeSummaries) return []
    
    const summaries = volumeData.value.volumeSummaries
    const totalVolume = summaries.reduce((sum: number, s: any) => sum + s.currentDollarVolume, 0)
    
    // Group into ranges
    const ranges = [
      { range: 'Top 5', start: 0, end: 5 },
      { range: '6-10', start: 5, end: 10 },
      { range: '11-20', start: 10, end: 20 },
      { range: '21-50', start: 20, end: 50 },
      { range: '51+', start: 50, end: Infinity }
    ]
    
    return ranges.map(({ range, start, end }) => {
      const coins = summaries
        .sort((a: any, b: any) => b.currentDollarVolume - a.currentDollarVolume)
        .slice(start, Math.min(end, summaries.length))
      
      const volume = coins.reduce((sum: number, c: any) => sum + c.currentDollarVolume, 0)
      
      return {
        range,
        coins: coins.map((c: any) => c.symbol),
        percentage: (volume / totalVolume) * 100,
        volume
      }
    }).filter(r => r.coins.length > 0)
  })
  
  // Computed hourly seasonality
  const hourlySeasonality = computed<HourlySeasonality[]>(() => {
    if (!rawData.value.length) return []
    
    // Group by hour
    const hourMap = new Map<number, { totalZ: number; count: number; volume: number }>()
    
    rawData.value.forEach(point => {
      const hour = new Date(point.timestamp * 1000).getUTCHours()
      const existing = hourMap.get(hour) || { totalZ: 0, count: 0, volume: 0 }
      
      existing.totalZ += point.zscore
      existing.count += 1
      existing.volume += point.volume || 0
      
      hourMap.set(hour, existing)
    })
    
    return Array.from(hourMap.entries())
      .map(([hour, data]) => ({
        hour,
        avgZScore: data.totalZ / data.count,
        volume: data.volume,
        count: data.count
      }))
      .sort((a, b) => a.hour - b.hour)
  })
  
  // Computed day of week effect
  const dayOfWeekEffect = computed<DayOfWeekEffect[]>(() => {
    if (!rawData.value.length) return []
    
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
    const dayMap = new Map<number, { totalZ: number; totalReturns: number; count: number; volume: number }>()
    
    rawData.value.forEach(point => {
      const day = new Date(point.timestamp * 1000).getUTCDay()
      const existing = dayMap.get(day) || { totalZ: 0, totalReturns: 0, count: 0, volume: 0 }
      
      existing.totalZ += point.zscore
      existing.totalReturns += point.returns1d
      existing.count += 1
      existing.volume += point.volume || 0
      
      dayMap.set(day, existing)
    })
    
    return days.map((day, dayNum) => {
      const data = dayMap.get(dayNum) || { totalZ: 0, totalReturns: 0, count: 1, volume: 0 }
      return {
        day,
        dayNum,
        avgZScore: data.totalZ / data.count,
        avgReturns: data.totalReturns / data.count,
        volume: data.volume
      }
    })
  })
  
  const fetchData = async () => {
    loading.value = true
    error.value = null
    
    try {
      // Fetch overview data for Z-scores
      const [overviewData, volLiqData] = await Promise.all([
        fetchZScoreOverview({
          exchange: exchange.value,
          timeframe: timeframe.value === '5min' ? '1h' : timeframe.value,
          period: period.value,
          topN: topN.value
        }),
        fetchVolatilityLiquidity({
          exchange: exchange.value,
          timeframe: timeframe.value === '1h' ? '1d' : timeframe.value,
          period: period.value,
          topN: topN.value
        })
      ])
            rawData.value = overviewData.zscoreTimeSeries || []
      volumeData.value = volLiqData
      
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
    
    // Data
    volumeFlow,
    rotationMatrix,
    liquidityConcentration,
    hourlySeasonality,
    dayOfWeekEffect,
    
    // Methods
    fetchData
  }
}