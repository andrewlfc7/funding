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

    <div class="dashboard-content">
      <MarketSignalsDashboard
        v-if="currentDashboard === 0"
        :key="`signals-${selectedExchange}-${selectedCoin}-${selectedPeriod}`"
      />
      <SignalPerformanceDashboard v-if="currentDashboard === 1" />
      <PortfolioConstructionDashboard v-if="currentDashboard === 2" />
      <TrendDirectionDashboard v-if="currentDashboard === 3" />
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
import { useTrendSignals } from '@/composables/useTrendSignals'

const dashboards = [
  { name: 'Signals' },
  { name: 'Signal Performance' },    
  { name: 'Portfolio Construction' },
  { name: 'Portfolio Exposure' },
  { name: 'Performance Attribution' }
]

const currentDashboard = ref(0)

const selectedCoin = ref('BTC')
const selectedExchange = ref<'binance' | string>('binance')
const selectedPeriod = ref<'30d' | '90d' | '180d' | '1y' | string>('90d')
const viewMode = ref<'combined' | 'individual'>('combined')

const {
  rawSignals, priceSeries, returnsSeries, volSeries, volumeSeries,
  loading, error, symbols, signalsBySymbol,
  aggregatedSignals, correlationMatrix,
  priceChartData, returnsChartData, volChartData, volumeEwmaChartData,
  fetchSignalsData,
} = useTrendSignals()

function periodToDays(p: string): number {
  const m = p.match(/^(\d+)([dwmy])$/)
  if (!m) return 90
  const n = Number(m[1]); const unit = m[2]
  switch (unit) {
    case 'd': return n
    case 'w': return n * 7
    case 'm': return n * 30
    case 'y': return n * 365
    default: return 90
  }
}

let reloadTimeout: number | null = null
async function reloadAll() {
  if (reloadTimeout) clearTimeout(reloadTimeout)
  reloadTimeout = setTimeout(async () => {
    const days = periodToDays(selectedPeriod.value)
    await fetchSignalsData({
      exchange: selectedExchange.value,
      market_type: 'spot',
      symbol: selectedCoin.value,
      days,
    })
  }, 0)
}

provide('controls', { selectedCoin, selectedExchange, selectedPeriod, viewMode, reloadAll })
provide('marketData', {
  priceSeries, returnsSeries, volSeries, volumeSeries, loading, error,
  priceChartData, returnsChartData, volChartData, volumeEwmaChartData,
  loadingLive: loading, errorLive: error,
})
provide('signalData', {
  xsecSignals: rawSignals,
  loading, error,
  series: signalsBySymbol,
  symbols, aggregatedSignals, correlationMatrix,
  loadingXSec: loading, errorXSec: error,
})
provide('utils', { periodToDays })

onMounted(reloadAll)
</script>
