// src/composables/useTrendSignals.ts
import { ref, computed } from 'vue'
import {
  fetchTrendSignals,
  fetchPriceSeries,
  fetchReturnsSeries,
  fetchVolSeries,
  fetchVolumeSeries,
  type TrendSignalPoint,
  type TimeSeriesPoint,
  type VolumePoint,
  type SignalsDataRequest
} from '@/api/trend/trend'

export function useTrendSignals() {
  // State
  const rawSignals = ref<TrendSignalPoint[]>([])
  const priceSeries = ref<TimeSeriesPoint[]>([])
  const returnsSeries = ref<TimeSeriesPoint[]>([])
  const volSeries = ref<TimeSeriesPoint[]>([])
  const volumeSeries = ref<VolumePoint[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  // Computed
  const symbols = computed(() => {
    const symbolSet = new Set(rawSignals.value.map(s => s.symbol))
    return Array.from(symbolSet).sort()
  })

  const timestamps = computed(() => {
    const tsSet = new Set(rawSignals.value.map(s => s.ts))
    return Array.from(tsSet).sort((a, b) => a - b)
  })

  const signalsBySymbol = computed(() => {
    const grouped: Record<string, TrendSignalPoint[]> = {}
    for (const signal of rawSignals.value) {
      if (!grouped[signal.symbol]) grouped[signal.symbol] = []
      grouped[signal.symbol].push(signal)
    }
    // sort copies (avoid mutating any shared arrays)
    Object.keys(grouped).forEach(sym => {
      grouped[sym] = [...grouped[sym]].sort((a, b) => a.ts - b.ts)
    })
    return grouped
  })

  const latestSignalsBySymbol = computed(() => {
    const latest: Record<string, TrendSignalPoint> = {}
    for (const signal of rawSignals.value) {
      if (!latest[signal.symbol] || signal.ts > latest[signal.symbol].ts) {
        latest[signal.symbol] = signal
      }
    }
    return latest
  })

  const aggregatedSignals = computed(() => {
    const byTs: Record<number, TrendSignalPoint[]> = {}
    for (const s of rawSignals.value) {
      if (!byTs[s.ts]) byTs[s.ts] = []
      byTs[s.ts].push(s)
    }
    return Object.entries(byTs)
      .map(([ts, arr]) => {
        const n = arr.length || 1
        return {
          ts: Number(ts),
          trend: arr.reduce((sum, s) => sum + s.trend, 0) / n,
          momentum: arr.reduce((sum, s) => sum + s.momentum, 0) / n,
          ewmac: arr.reduce((sum, s) => sum + s.ewmac, 0) / n,
          breakout: arr.reduce((sum, s) => sum + s.breakout, 0) / n,
          composite: arr.reduce((sum, s) => sum + s.composite, 0) / n,
        }
      })
      .sort((a, b) => a.ts - b.ts)
  })

  const correlationMatrix = computed(() => {
    const signals = ['trend', 'momentum', 'ewmac', 'breakout', 'composite'] as const
    const corr: Record<string, Record<string, number>> = {}
    if (!rawSignals.value.length) return corr
    const latest = Object.values(latestSignalsBySymbol.value)
    for (const s1 of signals) {
      corr[s1] = {}
      for (const s2 of signals) {
        if (s1 === s2) corr[s1][s2] = 1
        else {
          const v1 = latest.map(s => s[s1])
          const v2 = latest.map(s => s[s2])
          corr[s1][s2] = calculateCorrelation(v1, v2)
        }
      }
    }
    return corr
  })

  const priceChartData = computed(() => ({
    labels: priceSeries.value.map(p => p.date),
    values: priceSeries.value.map(p => p.value),
  }))
  const returnsChartData = computed(() => ({
    labels: returnsSeries.value.map(r => r.date),
    values: returnsSeries.value.map(r => r.value),
  }))
  const volChartData = computed(() => ({
    labels: volSeries.value.map(v => v.date),
    values: volSeries.value.map(v => v.value),
  }))
  const volumeEwmaChartData = computed(() => ({
    labels: volumeSeries.value.map(v => v.date),
    values: volumeSeries.value.map(v => v.dollar_volume_ewma || 0),
  }))
  const trendFactorData = computed(() => ({
    labels: aggregatedSignals.value.map(s => new Date(s.ts).toISOString().substring(0, 10)),
    values: aggregatedSignals.value.map(s => s.trend),
  }))
  const compositeSignalData = computed(() => ({
    labels: aggregatedSignals.value.map(s => new Date(s.ts).toISOString().substring(0, 10)),
    trend: aggregatedSignals.value.map(s => s.trend),
    momentum: aggregatedSignals.value.map(s => s.momentum),
    ewmac: aggregatedSignals.value.map(s => s.ewmac),
    breakout: aggregatedSignals.value.map(s => s.breakout),
    composite: aggregatedSignals.value.map(s => s.composite),
  }))

  function calculateCorrelation(x: number[], y: number[]): number {
    if (x.length !== y.length || x.length < 2) return 0
    const n = x.length
    const sumX = x.reduce((a, b) => a + b, 0)
    const sumY = y.reduce((a, b) => a + b, 0)
    const sumXY = x.reduce((acc, xi, i) => acc + xi * y[i], 0)
    const sumX2 = x.reduce((acc, xi) => acc + xi * xi, 0)
    const sumY2 = y.reduce((acc, yi) => acc + yi * yi, 0)
    const numerator = n * sumXY - sumX * sumY
    const denominator = Math.sqrt((n * sumX2 - sumX * sumX) * (n * sumY2 - sumY * sumY))
    return denominator === 0 ? 0 : numerator / denominator
  }

  async function fetchSignalsData(params: SignalsDataRequest) {
    loading.value = true
    error.value = null
    try {
      const symbolForTimeSeries = params.symbol || 'BTC'
      const promises = [
        fetchTrendSignals(params),
        fetchPriceSeries({ ...params, symbol: symbolForTimeSeries }),
        fetchReturnsSeries({ ...params, symbol: symbolForTimeSeries }),
        fetchVolSeries({ ...params, symbol: symbolForTimeSeries }),
        fetchVolumeSeries({ ...params, symbol: symbolForTimeSeries, span: 60 }),
      ] as const
      const [signals, prices, returns, vol, volume] = await Promise.all(promises)
      rawSignals.value = signals
      priceSeries.value = prices
      returnsSeries.value = returns
      volSeries.value = vol
      volumeSeries.value = volume
      console.debug('[TrendSignals] Data loaded:', {
        signals: signals.length,
        symbols: new Set(signals.map(s => s.symbol)).size,
        prices: prices.length, returns: returns.length, vol: vol.length, volume: volume.length,
        selectedSymbol: symbolForTimeSeries
      })
    } catch (e: any) {
      error.value = e?.message ?? 'Failed to fetch trend signals data'
      console.error('[TrendSignals] Error:', e)
    } finally {
      loading.value = false
    }
  }

  function clear() {
    rawSignals.value = []
    priceSeries.value = []
    returnsSeries.value = []
    volSeries.value = []
    volumeSeries.value = []
    error.value = null
  }

  return {
    // raw
    rawSignals, priceSeries, returnsSeries, volSeries, volumeSeries,
    // state
    loading, error,
    // computed
    symbols, timestamps, signalsBySymbol, latestSignalsBySymbol,
    aggregatedSignals, correlationMatrix,
    // chart helpers
    priceChartData, returnsChartData, volChartData, volumeEwmaChartData,
    trendFactorData, compositeSignalData,
    // methods
    fetchSignalsData, clear,
    hasData: computed(() => rawSignals.value.length > 0),
  }
}