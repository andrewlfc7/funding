<template>
  <div class="dashboard-header">
    <h2>Volatility Rankings</h2>
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
    <!-- Annualized Volatility Heatmap -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Annualized Volatility Heatmap</h3>
      </div>
      <div class="card-content">
        <div class="vol-heatmap-container">
          <div v-if="annualizedVolatilityData.length > 0" class="vol-grid">
            <div 
              v-for="item in annualizedVolatilityData" 
              :key="item.symbol"
              class="vol-cell"
              :style="{ 
                backgroundColor: getVolatilityHeatmapColor(item.annualizedVol),
                order: item.rank
              }"
              :title="`${item.symbol}: ${item.annualizedVol.toFixed(1)}% (Z: ${item.volatilityZScore.toFixed(2)})`"
            >
              <span class="symbol">{{ item.symbol }}</span>
              <span class="value">{{ item.annualizedVol.toFixed(0) }}%</span>
            </div>
          </div>
          <div class="heatmap-legend">
            <span class="legend-label">Low</span>
            <div class="gradient-bar"></div>
            <span class="legend-label">High</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 1-Hour Volatility vs Volume Z-Score -->
    <div class="metric-card">
      <div class="card-header">
        <h3>1-Hour Volatility vs Volume Z-Score</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <ScatterChart
            v-if="volVsVolumeZScore1H.length > 0"
            :data="volVsVolumeZScore1H"
            x-field="volumeZScore"
            y-field="volatility"
            label-field="symbol"
            x-label="Volume Z-Score"
            y-label="1H Volatility (%)"
            :show-labels="true"
            :show-quadrants="true"
          />
        </div>
      </div>
    </div>

    <!-- 1-Day Volatility vs Volume Z-Score -->
    <div class="metric-card">
      <div class="card-header">
        <h3>1-Day Volatility vs Volume Z-Score</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <ScatterChart
            v-if="volVsVolumeZScore1D.length > 0"
            :data="volVsVolumeZScore1D"
            x-field="volumeZScore"
            y-field="volatility"
            label-field="symbol"
            x-label="Volume Z-Score"
            y-label="1D Volatility (%)"
            :show-labels="true"
            :show-quadrants="true"
          />
        </div>
      </div>
    </div>

    <!-- Volume Z-Score vs Returns -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Volume Z-Score vs Returns</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <ScatterChart
            v-if="volumeZScoreVsReturns.length > 0"
            :data="volumeZScoreVsReturns"
            x-field="returns"
            y-field="volumeZScore"
            label-field="symbol"
            size-field="volatilityZScore"
            x-label="Returns (%)"
            y-label="Volume Z-Score"
            :color-field="'volatilityZScore'"
            :show-labels="true"
            :show-quadrants="true"
          />
        </div>
      </div>
    </div>

    <!-- Volatility Rankings Table -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Volatility Rankings</h3>
      </div>
      <div class="card-content">
        <div class="vol-rankings-table">
          <div class="rankings-section">
            <h4>High Volatility (Z > 1.5)</h4>
            <table class="rankings-table">
              <thead>
                <tr>
                  <th>Symbol</th>
                  <th>Ann. Vol</th>
                  <th>Vol Z-Score</th>
                  <th>Volume Z-Score</th>
                  <th>Returns</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="coin in highVolCoins" :key="coin.symbol">
                  <td>{{ coin.symbol }}</td>
                  <td>{{ coin.annualizedVol.toFixed(1) }}%</td>
                  <td class="z-score high">{{ coin.volatilityZScore.toFixed(2) }}σ</td>
                  <td>{{ coin.volumeZScore.toFixed(2) }}σ</td>
                  <td :class="coin.returns > 0 ? 'positive' : 'negative'">
                    {{ coin.returns > 0 ? '+' : '' }}{{ coin.returns.toFixed(2) }}%
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          
          <div class="rankings-section">
            <h4>Low Volatility (Z < -1.5)</h4>
            <table class="rankings-table">
              <thead>
                <tr>
                  <th>Symbol</th>
                  <th>Ann. Vol</th>
                  <th>Vol Z-Score</th>
                  <th>Volume Z-Score</th>
                  <th>Returns</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="coin in lowVolCoins" :key="coin.symbol">
                  <td>{{ coin.symbol }}</td>
                  <td>{{ coin.annualizedVol.toFixed(1) }}%</td>
                  <td class="z-score low">{{ coin.volatilityZScore.toFixed(2) }}σ</td>
                  <td>{{ coin.volumeZScore.toFixed(2) }}σ</td>
                  <td :class="coin.returns > 0 ? 'positive' : 'negative'">
                    {{ coin.returns > 0 ? '+' : '' }}{{ coin.returns.toFixed(2) }}%
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useVolatilityData } from '@/composables/useVolatilityAnalysis'
import { useMetaData } from '@/composables/useMetaData'
import MetricCard from '../components/common/MetricCard.vue'
import ScatterChart from '../components/charts/ScatterChart.vue'

// Meta data
const { exchanges, defaultExchange, loading: metaLoading, loadMetaData } = useMetaData()

// Controls
const topN = ref(50)
const timeframe = ref('1h')
const period = ref('30d')
const exchange = ref('')

// Use the volatility composable
const {
  loading,
  error,
  annualizedVolatilityData,
  volVsVolumeZScore1H,
  volVsVolumeZScore1D,
  volumeZScoreVsReturns,
  highVolCoins,
  lowVolCoins,
  getVolatilityHeatmapColor,
  fetchData: fetch
} = useVolatilityData()

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
