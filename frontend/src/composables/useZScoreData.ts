import { ref, computed } from 'vue'
import {
  fetchZScoreOverview,
  zrowToSeries,          // new adapter
  type ZRow,            // new data point type
  getLatestBySymbol,
  groupDataBySymbol,
} from '@/api/zscore'

export function useZScoreData() {
  // State
  const rawData = ref<ZRow[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Group data by symbol
  const dataBySymbol = computed(() => groupDataBySymbol(rawData.value))
  const latestBySymbol = computed(() => getLatestBySymbol(rawData.value))

  // Strongly typed entries to avoid 'unknown'
  const latestEntries = computed(
    () => Array.from(latestBySymbol.value.entries()) as Array<[string, ZRow]>
  )

  // All unique symbols
  const symbols = computed(() => Array.from(dataBySymbol.value.keys()).sort())

  // Chart data (seconds -> ms conversion handled in zrowToSeries)
  const chartData = computed(() => zrowToSeries(rawData.value))

  // ---------- Dashboard: Z-Score Overview ----------

  // 1) Z-Score vs 1D Returns
  const zscoreVsReturns1d = computed(() =>
    latestEntries.value.map(([symbol, d]) => ({
      symbol,
      zscore: d.zscore,
      returns: d.returns1d * 100, // %
    }))
  )

  // 2) Z-Score vs 1H Log Returns
  const zscoreVsLogReturns1h = computed(() =>
    latestEntries.value.map(([symbol, d]) => ({
      symbol,
      zscore: d.zscore,
      logReturns: d.logReturns1h,
    }))
  )

  // 3) Z-Score vs Rolling Volume (USD, in millions)
  const zscoreVsRollingVolume = computed(() =>
    latestEntries.value.map(([symbol, d]) => ({
      symbol,
      zscore: d.zscore,
      rollingVolume:
        ((d.rollingDollarVolume ??
          // (older payload fallback—safe to keep)
          (d as any).rollingVolume ??
          0) as number) / 1e6,
    }))
  )

  // 4) Z-Score Distribution (latest per symbol)
  const zscoreDistribution = computed(() => {
    const allZ = latestEntries.value.map(([, d]) => d.zscore)
    if (!allZ.length) return null

    const edges = [-3, -2.5, -2, -1.5, -1, -0.5, 0, 0.5, 1, 1.5, 2, 2.5, 3]
    const counts = Array(edges.length - 1).fill(0)

    for (const z of allZ) {
      for (let i = 0; i < edges.length - 1; i++) {
        const left = edges[i], right = edges[i + 1]
        const isLast = i === edges.length - 2
        if ((z >= left && z < right) || (isLast && z === right)) {
          counts[i]++
          break
        }
      }
    }

    return {
      buckets: edges.slice(0, -1).map((b, i) => `${b} to ${edges[i + 1]}`),
      counts,
    }
  })

  // Oversold (z < -2)
  const oversoldCoins = computed(() =>
    latestEntries.value
      .filter(([, d]) => d.zscore < -2)
      .sort((a, b) => a[1].zscore - b[1].zscore)
      .slice(0, 10)
      .map(([symbol, d], idx) => ({
        rank: idx + 1,
        symbol,
        zscore: d.zscore,
        returns1d: d.returns1d,
        volume: d.rollingDollarVolume ?? (d as any).rollingVolume ?? 0,
      }))
  )

  // Overbought (z > 2)
  const overboughtCoins = computed(() =>
    latestEntries.value
      .filter(([, d]) => d.zscore > 2)
      .sort((a, b) => b[1].zscore - a[1].zscore)
      .slice(0, 10)
      .map(([symbol, d], idx) => ({
        rank: idx + 1,
        symbol,
        zscore: d.zscore,
        returns1d: d.returns1d,
        volume: d.rollingDollarVolume ?? (d as any).rollingVolume ?? 0,
      }))
  )

  // Aggregate stats across latest z-scores
  const stats = computed(() => {
    const all = latestEntries.value.map(([, d]) => d.zscore)
    if (!all.length) return { mean: 0, std: 0, min: 0, max: 0, current: 0 }

    const mean = all.reduce((a, b) => a + b, 0) / all.length
    const variance = all.reduce((a, b) => a + (b - mean) ** 2, 0) / all.length
    const std = Math.sqrt(variance)
    const min = Math.min(...all)
    const max = Math.max(...all)
    return { mean, std, min, max, current: mean }
  })

  // Latest datapoint overall (mostly for backwards compat)
  const latest = computed(() => (rawData.value.length ? rawData.value[rawData.value.length - 1] : null))

  // Fetch data (single-coin if baseCoin is provided; otherwise universe via topN)
  async function fetchData(params: {
    baseCoin?: string
    timeframe: '1h' | '4h' | '1d'
    period: string
    exchange: string
    marketType?: 'spot' | 'perps'
    topN?: number
  }) {
    loading.value = true
    error.value = null
    try {
      const response = await fetchZScoreOverview({
        exchange: params.exchange,
        marketType: params.marketType ?? 'spot',
        timeframe: params.timeframe,
        period: params.period,
        baseCoin: params.baseCoin, // if omitted => universe mode
        topN: params.topN ?? 50,
      })
      rawData.value = response.zscoreTimeSeries ?? []
    } catch (e: any) {
      error.value = e?.message ?? 'Failed to fetch Z-Score data'
      console.error('Z-Score fetch error:', e)
    } finally {
      loading.value = false
    }
  }

  function clear() {
    rawData.value = []
    error.value = null
  }

  return {
    // State
    data: chartData,
    rawData,
    loading,
    error,

    // Multi-coin helpers
    dataBySymbol,
    latestBySymbol,
    symbols,

    // Plots
    zscoreVsReturns1d,
    zscoreVsLogReturns1h,
    zscoreVsRollingVolume,
    zscoreDistribution,
    oversoldCoins,
    overboughtCoins,

    // Stats
    stats,
    latest,
    hasData: computed(() => rawData.value.length > 0),

    // Methods
    fetchData,
    clear,
  }
}
