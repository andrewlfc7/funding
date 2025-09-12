<template>
  <!-- Remove the wrapper div -->
  <div class="dashboard-header">
    <h2>Z-Score Overview</h2>
    <div class="header-controls">
      <select v-model="topN" @change="fetchData">
        <option :value="30">Top 30</option>
        <option :value="50">Top 50</option>
        <option :value="100">Top 100</option>
      </select>
      <select v-model="timeframe" @change="fetchData">
        <option value="1h">1H</option>
        <option value="4h">4H</option>
        <option value="1d">1D</option>
      </select>
      <select v-model="period" @change="fetchData">
        <option value="7d">7 Days</option>
        <option value="30d">30 Days</option>
        <option value="90d">90 Days</option>
      </select>
      <select v-model="exchange" @change="fetchData" :disabled="metaLoading">
        <option v-for="ex in exchanges" :key="ex" :value="ex">
          {{ ex }}
        </option>
      </select>
      <button @click="fetchData" class="update-btn" :disabled="loading || metaLoading">
        {{ loading ? 'Loading...' : 'Update' }}
      </button>
    </div>
  </div>

  <div v-if="!metaLoading" class="dashboard-grid">
    <!-- 1. Z-Score vs 1D Returns -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Z-Score vs 1D Returns</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <ScatterChart
            v-if="zscoreVsReturns1d.length > 0"
            :data="zscoreVsReturns1d"
            x-field="zscore"
            y-field="returns"
            label-field="symbol"
            x-label="Z-Score"
            y-label="1D Returns (%)"
            :show-labels="true"
          />
        </div>
      </div>
    </div>

    <!-- 2. Z-Score vs 1H Log Returns -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Z-Score vs 1H Log Returns</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <ScatterChart
            v-if="zscoreVsLogReturns1h.length > 0"
            :data="zscoreVsLogReturns1h"
            x-field="zscore"
            y-field="logReturns"
            label-field="symbol"
            x-label="Z-Score"
            y-label="1H Log Returns"
            :show-labels="true"
          />
        </div>
      </div>
    </div>

    <!-- 3. Z-Score vs Rolling Volume -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Z-Score vs Rolling Volume</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <ScatterChart
            v-if="zscoreVsRollingVolume.length > 0"
            :data="zscoreVsRollingVolume"
            x-field="zscore"
            y-field="rollingVolume"
            label-field="symbol"
            x-label="Z-Score"
            y-label="20-MA Volume (M USD)"
            :show-labels="true"
          />
        </div>
      </div>
    </div>

    <!-- 4. Z-Score Distribution -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Z-Score Distribution (All Coins)</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <HistogramChart
            v-if="zscoreDistribution"
            :data="zscoreDistribution"
          />
        </div>
      </div>
    </div>

    <!-- 5. Z-Score Time Series (full width) -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Z-Score Time Series</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <TimeSeriesChart
            v-if="hasData"
            :data="chartData"
            :y-field="'value'"
            :y-label="'Z-Score'"
            :group-by="'symbol'"
          />
        </div>
      </div>
    </div>

    <!-- Rankings Tables -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Z-Score Rankings</h3>
      </div>
      <div class="card-content">
        <div class="rankings-table">
          <div class="table-section">
            <h4>Oversold (Z < -2)</h4>
            <div class="coin-list">
              <div v-for="coin in oversoldCoins" :key="coin.symbol" class="coin-row">
                <span class="rank">{{ coin.rank }}</span>
                <span class="symbol">{{ coin.symbol }}</span>
                <span class="zscore negative">{{ coin.zscore.toFixed(2) }}</span>
                <span class="returns" :class="coin.returns1d > 0 ? 'positive' : 'negative'">
                  {{ coin.returns1d > 0 ? '+' : ''}}{{ (coin.returns1d * 100).toFixed(1) }}%
                </span>
                <span class="volume">${{ formatNumber(coin.volume) }}</span>
              </div>
            </div>
          </div>
          
          <div class="table-section">
            <h4>Overbought (Z > 2)</h4>
            <div class="coin-list">
              <div v-for="coin in overboughtCoins" :key="coin.symbol" class="coin-row">
                <span class="rank">{{ coin.rank }}</span>
                <span class="symbol">{{ coin.symbol }}</span>
                <span class="zscore positive">+{{ coin.zscore.toFixed(2) }}</span>
                <span class="returns" :class="coin.returns1d > 0 ? 'positive' : 'negative'">
                  {{ coin.returns1d > 0 ? '+' : ''}}{{ (coin.returns1d * 100).toFixed(1) }}%
                </span>
                <span class="volume">${{ formatNumber(coin.volume) }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useZScoreData } from '@/composables/useZScoreData'
import { useMetaData } from '@/composables/useMetaData'
import { formatNumber } from '@/utils/formatters'
import MetricCard from '../components/common/MetricCard.vue'
import ScatterChart from '../components/charts/ScatterChart.vue'
import HistogramChart from '../components/charts/HistogramChart.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'
import LoadingSpinner from '../components/common/LoadingSpinner.vue'

// Meta data
const { exchanges, defaultExchange, loading: metaLoading, loadMetaData } = useMetaData()


// Controls
const topN = ref(50)
const period = ref('30d')
const timeframe = ref<'1h' | '4h' | '1d'>('1h') 

const exchange = ref('')

// Use the Z-score composable with multi-coin support
const { 
  data: chartData,
  loading, 
  error, 
  hasData,
  zscoreVsReturns1d,
  zscoreVsLogReturns1h,
  zscoreVsRollingVolume,
  zscoreDistribution,
  oversoldCoins,
  overboughtCoins,
  fetchData: fetch 
} = useZScoreData()

async function fetchData() {
  if (!exchange.value) return
  
  await fetch({
    timeframe: timeframe.value,
    period: period.value,
    exchange: exchange.value,
    topN: topN.value
  })
}

// Initialize on mount
onMounted(async () => {
  await loadMetaData('spot')
  exchange.value = defaultExchange.value
  await fetchData()
})
</script>
