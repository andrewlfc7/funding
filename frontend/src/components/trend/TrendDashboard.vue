<template>
  <div class="trend-dashboard">
    <nav class="dashboard-nav">
      <button 
        v-for="(dashboard, index) in dashboards" 
        :key="dashboard.name"
        @click="currentDashboard = index"
        :class="['nav-btn', { active: currentDashboard === index }]"
        :title="dashboard.name"
      >
        <span class="nav-indicator">{{ (index + 1).toString().padStart(2, '0') }}</span>
        <span class="nav-text">{{ dashboard.name }}</span>
        <span class="nav-tooltip">{{ dashboard.name }}</span>
      </button>
    </nav>

    <!-- Dashboard content area -->
    <div class="dashboard-content">
      <!-- Dashboard 1: Market Data & Factor Signals -->
      <MarketSignalsDashboard v-if="currentDashboard === 0" />
      
      <!-- Dashboard 2: Signal Performance & Risk Analytics -->
      <SignalPerformanceDashboard v-if="currentDashboard === 1" />
      
      <!-- Dashboard 3: Portfolio Construction & Risk Management -->
      <PortfolioConstructionDashboard v-if="currentDashboard === 2" />
      
      <!-- Dashboard 4: Portfolio Trend & Direction Analysis -->
      <TrendDirectionDashboard v-if="currentDashboard === 3" />
      
      <!-- Dashboard 5: Performance Attribution & P&L Analysis -->
      <PerformanceAttributionDashboard v-if="currentDashboard === 4" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, provide, onMounted } from 'vue'
import MarketSignalsDashboard from './dashboards/MarketSignals.vue'
import SignalPerformanceDashboard from './dashboards/SignalPerformance.vue'
import PortfolioConstructionDashboard from './dashboards/PortfolioConstruction.vue'
import TrendDirectionDashboard from './dashboards/PortfolioExposure.vue'
import PerformanceAttributionDashboard from './dashboards/PerformanceAttribution.vue'

import { useXSecSignals } from '@/composables/useXSecSignals'
import { fetchKlines, fetchReturns, fetchVolatility } from '@/api/trend/volatility'

const dashboards = [
  { name: 'Signals' },
  { name: 'Signal Performance' },    
  { name: 'Portfolio Construction' },
  { name: 'Portfolio Exposure' },
  { name: 'Performance Attribution' }
]

// Current dashboard state
const currentDashboard = ref(0)

// Shared controls state
const selectedCoin = ref('BTC')
const selectedExchange = ref<'binance' | string>('binance')
const selectedPeriod = ref<'30d' | '60d' | '90d' | string>('90d')
const viewMode = ref<'combined' | 'individual'>('combined')

// Shared data state - X-sec signals
const {
  signals: xsecSignals, 
  loading: loadingXSec, 
  error: errorXSec,
  selectedSymbol, 
  symbols, 
  series, 
  load: loadXSec,
} = useXSecSignals()

// Shared data state - Live market data
const klines = ref<any[]>([])
const returnsSeries = ref<{ ts: number; value: number }[]>([])
const volSeries = ref<{ ts: number; value: number }[]>([])
const loadingLive = ref(false)
const errorLive = ref<string | null>(null)

// Utility functions
function periodToDays(p: string): number {
  const m = p.match(/^(\d+)([dw])$/)
  if (!m) return 180
  const n = Number(m[1]); const u = m[2]
  return u === 'd' ? n : n * 7
}


async function loadLive() {
  loadingLive.value = true
  errorLive.value = null
  try {
    const days = periodToDays(selectedPeriod.value)
    const symbol = selectedCoin.value

    const [k, r, v] = await Promise.all([
      fetchKlines({ exchange: selectedExchange.value, market_type: 'spot', symbol, days }),
      fetchReturns({ exchange: selectedExchange.value, market_type: 'spot', symbol, days }),
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

// Load all data
async function reloadAll() {
  const days = periodToDays(selectedPeriod.value)

  await loadXSec({
    exchange: selectedExchange.value,
    market_type: 'spot',
    days,
    vol_window: 30,
    min_decile: 3,
  })

  await loadLive()
}

// Provide shared state to all dashboards
provide('controls', {
  selectedCoin,
  selectedExchange, 
  selectedPeriod,
  viewMode,
  reloadAll
})

provide('marketData', {
  klines,
  returnsSeries,
  volSeries,
  loadingLive,
  errorLive
})

provide('signalData', {
  xsecSignals,
  loadingXSec,
  errorXSec,
  series,
  symbols
})

provide('utils', {
  periodToDays,
})

onMounted(reloadAll)
</script>
