<template>
  <div class="trend-page">
    <div class="trend-header">
      <h1>Trend Analysis</h1>
      <Controls
        v-model:coin="selectedCoin"
        v-model:exchange="selectedExchange"
        v-model:period="selectedPeriod"
        v-model:viewMode="viewMode"
        :availableCoins="availableCoinsUI"
        :availableExchanges="availableExchangesUI"
        @update="reloadAll"
      />
    </div>

    <div class="charts-layout">
      <!-- Candles -->
      <CandlestickChart
        :data="{ klines, loading: loadingLive, error: errorLive }"
        :coin="pairSymbol"
        :period="selectedPeriod"
        class="main-chart-section compact"
      />

      <!-- Signals -->
      <template v-if="viewMode === 'combined'">
        <CombinedSignalsChart
          :labels="series.labels"
          :dataById="series.dataById"
          :signals="xsecSignals"
          v-model:activeSignals="activeSignals"
          :loading="loadingXSec"
          :error="errorXSec"
        />
      </template>

      <template v-else>
        <div class="grid grid-cols-2 gap-4">
          <IndividualSignalChart
            title="Momentum"
            color="rgb(54, 162, 235)"
            :labels="series.labels"
            :values="series.dataById['momentum'] ?? []"
            :loading="loadingXSec"
            :error="errorXSec"
          />
          <IndividualSignalChart
            title="EWMA"
            color="rgb(75, 192, 192)"
            :labels="series.labels"
            :values="series.dataById['ewmac'] ?? []"
            :loading="loadingXSec"
            :error="errorXSec"
          />
          <IndividualSignalChart
            title="Breakout"
            color="rgb(153, 102, 255)"
            :labels="series.labels"
            :values="series.dataById['breakout'] ?? []"
            :loading="loadingXSec"
            :error="errorXSec"
          />
          <IndividualSignalChart
            title="Composite"
            color="rgb(255, 159, 64)"
            :labels="series.labels"
            :values="series.dataById['composite'] ?? []"
            :loading="loadingXSec"
            :error="errorXSec"
          />
        </div>
      </template>

      <!-- Returns / Vol (split) -->
      <ReturnsVolatilityChart
        :returns="returnsSeries"
        :volatility="volSeries"
        :loading="loadingLive"
        :error="errorLive"
        v-model:modelValue="returnMetric"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, onMounted } from 'vue'

import Controls from './Controls.vue'
import CandlestickChart from './CandlestickChart.vue'
import CombinedSignalsChart from './CombinedSignalsChart.vue'
import IndividualSignalChart from './IndividualSignalChart.vue'
import ReturnsVolatilityChart from './ReturnsVolatilityChart.vue'

import { useXSecSignals } from '@/composables/useXSecSignals'
import { fetchKlines, fetchReturns, fetchVolatility } from '@/api/signals/volatility'
import { xsecSignals } from '@/utils/xsecSignals'

// --- Controls / state ---
const selectedCoin = ref('BTC')
const selectedExchange = ref<'binance' | string>('binance')
const selectedPeriod = ref<'30d' | '60d' | '90d' | string>('90d')
const viewMode = ref<'combined' | 'individual'>('combined')
const returnMetric = ref<'returns' | 'volatility' | 'both'>('returns')

// If you still want to show pair label on candle title:
const pairSymbol = computed(() => `${selectedCoin.value.toUpperCase()}USDT`)

// --- Local candles + split RV ---
const klines = ref<any[]>([])
const returnsSeries = ref<{ ts: number; value: number }[]>([])
const volSeries = ref<{ ts: number; value: number }[]>([])
const loadingLive = ref(false)
const errorLive = ref<string | null>(null)

// --- X-sec signals ---
const {
  // RENAMED rows -> signals
  signals, loading: loadingXSec, error: errorXSec,
  selectedSymbol, symbols, series, load: loadXSec,
} = useXSecSignals()

// Build coin list from x-sec rows (base symbols). Fallback to current selection.
const QUOTES = ['USDT', 'USDC', 'FDUSD', 'TUSD', 'BUSD', 'USD', 'DAI', 'USDP']
function toBaseSymbol(s: string) {
  if (!s) return ''
  const up = s.toUpperCase().replace('/', '').trim()
  for (const q of QUOTES) if (up.endsWith(q)) return up.slice(0, -q.length)
  return up
}
const availableCoinsUI = computed(() => {
  const set = new Set<string>()
  // UPDATED: iterate signals instead of rows
  for (const r of signals.value) set.add(toBaseSymbol(r.symbol))
  const arr = Array.from(set).sort()
  return arr.length ? arr : [selectedCoin.value]
})
const availableExchangesUI = computed(() => ['binance'])

// Keep selections in sync
watch(selectedCoin, (c) => { selectedSymbol.value = c })
watch(selectedSymbol, (s) => { if (s && s !== selectedCoin.value) selectedCoin.value = s })

// Toggles (no 'trend')
const activeSignals = reactive<Record<string, boolean>>({
  momentum: true, ewmac: true, breakout: true, composite: true
})

function periodToDays(p: string): number {
  const m = p.match(/^(\d+)([dw])$/)
  if (!m) return 180
  const n = Number(m[1]); const u = m[2]
  return u === 'd' ? n : n * 7
}

// --- loaders ---
async function loadLive() {
  loadingLive.value = true
  errorLive.value = null
  try {
    const days = periodToDays(selectedPeriod.value)
    const symbol = selectedCoin.value.toUpperCase() // base

    const [k, r, v] = await Promise.all([
      fetchKlines({   exchange: selectedExchange.value, market_type: 'spot', symbol, days }),
      fetchReturns({  exchange: selectedExchange.value, market_type: 'spot', symbol, days }),
      fetchVolatility({ exchange: selectedExchange.value, market_type: 'spot', symbol, days, vol_window: 30 }),
    ])

    klines.value = k
    returnsSeries.value = r
    volSeries.value = v

    console.debug('[Live]', { symbol, klines: k.length, returns: r.length, vol: v.length })
  } catch (e: any) {
    errorLive.value = e?.message ?? String(e)
    console.error('[Live] error:', e)
  } finally {
    loadingLive.value = false
  }
}

async function reloadAll() {
  const days = periodToDays(selectedPeriod.value)

  // IMPORTANT:
  // 1) Load X-SEC WITHOUT symbol to populate the full universe for the dropdown & charts
  // 2) Then load live series for the currently selected coin
  await loadXSec({
    exchange: selectedExchange.value,
    market_type: 'spot',
    days,
    vol_window: 30,
    min_decile: 3,
    // no symbol here → backend returns ALL eligible symbols
  })

  await loadLive()
}

onMounted(reloadAll)
</script>

<style scoped>
.trend-page { display: flex; flex-direction: column; gap: 16px; }
.trend-header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
.charts-layout { display: grid; gap: 16px; }
.grid { display: grid; }
.grid-cols-2 { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.gap-4 { gap: 16px; }
.main-chart-section.compact { min-height: 320px; }
</style>
