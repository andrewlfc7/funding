import { ref, computed } from 'vue'
import { fetchXSecSignals, buildSeries, type XSecParams, type XSecRow } from '@/api/signals/xsec'
import { xsecSignals } from '@/utils/xsecSignals'

const DEBUG = false

export function useXSecSignals() {
  // RENAMED rows -> signals
  const signals = ref<XSecRow[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const selectedSymbol = ref<string>('') // base symbol you want to chart

  async function load(params: XSecParams) {
    loading.value = true
    error.value = null
    try {
      const data = await fetchXSecSignals(params)

      // Normalize: backend returns a flat array now, but we also tolerate {rows:[]}
      const arr: XSecRow[] = Array.isArray(data) ? data : Array.isArray((data as any)?.rows) ? (data as any).rows : []
      signals.value = arr

      if (DEBUG) console.debug('[xsec] fetched signals:', signals.value.length, signals.value.slice(0, 3))

      // If nothing selected yet, default to first available symbol from the response
      if (!selectedSymbol.value && symbols.value.length) {
        selectedSymbol.value = symbols.value[0]
        if (DEBUG) console.debug('[xsec] defaulted selectedSymbol ->', selectedSymbol.value)
      }
    } catch (e: any) {
      error.value = e?.message ?? String(e)
      if (DEBUG) console.error('[xsec] load error:', error.value)
    } finally {
      loading.value = false
    }
  }

  // Build symbol list from the current signals
  const symbols = computed(() =>
    Array.from(new Set(signals.value.map(r => r.symbol))).sort()
  )

  // Build time-series for the currently selected symbol
  const series = computed(() => {
    if (!selectedSymbol.value) {
      return { labels: [] as Date[], dataById: {} as Record<string, number[]> }
    }
    const ids = xsecSignals.map(s => s.id) // ['trend','momentum','ewmac','breakout','composite']
    const s = buildSeries(signals.value, selectedSymbol.value, ids)
    if (DEBUG) {
      const lens = Object.fromEntries(Object.entries(s.dataById).map(([k, v]) => [k, v.length]))
      console.debug('[xsec] series for', selectedSymbol.value, 'points:', s.labels.length, lens)
    }
    return s
  })

  return {
    // EXPORTED names updated
    signals, loading, error,
    selectedSymbol, symbols, series,
    load,
  }
}
