<template>
  <div class="zscore-overview">
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
      <MetricCard 
        title="Z-Score vs 1D Returns"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
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
      </MetricCard>

      <!-- 2. Z-Score vs 1H Log Returns -->
      <MetricCard 
        title="Z-Score vs 1H Log Returns"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
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
      </MetricCard>

      <!-- 3. Z-Score vs Rolling Volume -->
      <MetricCard 
        title="Z-Score vs Rolling Volume"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
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
      </MetricCard>

      <!-- 4. Z-Score Distribution -->
      <MetricCard 
        title="Z-Score Distribution (All Coins)"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <HistogramChart
          v-if="zscoreDistribution"
          :data="zscoreDistribution"
        />
      </MetricCard>

      <!-- 5. Z-Score Time Series (full width) -->
      <MetricCard 
        title="Z-Score Time Series"
        :span="2"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <TimeSeriesChart
          v-if="hasData"
          :data="chartData"
          :y-field="'value'"
          :y-label="'Z-Score'"
          :group-by="'symbol'"
        />
      </MetricCard>

      <!-- Rankings Tables -->
      <MetricCard 
        title="Z-Score Rankings"
        :span="2"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
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
      </MetricCard>
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
const timeframe = ref('1h')
const period = ref('30d')
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

<style scoped>
/* Keep existing styles */
.rankings-table {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-lg);
}

.table-section h4 {
  margin: 0 0 var(--space-md) 0;
  color: var(--text-secondary);
  font-size: 14px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.coin-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.coin-row {
  display: grid;
  grid-template-columns: 30px 80px 80px 80px 100px;
  align-items: center;
  padding: 8px;
  background: var(--bg-primary);
  border-radius: 6px;
  font-size: 14px;
}

.coin-row .rank {
  color: var(--text-secondary);
}

.coin-row .symbol {
  font-weight: 600;
  color: var(--text-primary);
}

.coin-row .zscore {
  font-weight: 500;
}

.coin-row .zscore.positive {
  color: var(--color-positive);
}

.coin-row .zscore.negative {
  color: var(--color-negative);
}

.coin-row .returns {
  text-align: right;
  font-size: 13px;
}

.coin-row .returns.positive {
  color: var(--color-positive);
}

.coin-row .returns.negative {
  color: var(--color-negative);
}

.coin-row .volume {
  text-align: right;
  color: var(--text-secondary);
  font-size: 13px;
}

@media (max-width: 768px) {
  .rankings-table {
    grid-template-columns: 1fr;
  }
  
  .coin-row {
    grid-template-columns: 30px 60px 60px 60px 80px;
    font-size: 12px;
  }
}
</style>