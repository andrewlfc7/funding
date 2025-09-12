<template>
  <!-- Remove wrapper div -->
  <div class="dashboard-header">
    <h2>Cross-Asset Analytics</h2>
    <div class="header-controls">
      <select v-model="indexCoin" @change="fetchData">
        <option value="BTC">BTC</option>
        <option value="ETH">ETH</option>
        <option value="SOL">SOL</option>
      </select>
      <select v-model="topN" @change="fetchData">
        <option :value="5">Top 5</option>
        <option :value="10">Top 10</option>
        <option :value="20">Top 20</option>
        <option :value="30">Top 30</option>
        <option :value="50">Top 50</option>
      </select>
      <select v-model="period" @change="fetchData">
        <option value="24h">24 Hours</option>
        <option value="7d">7 Days</option>
        <option value="30d">30 Days</option>
        <option value="90d">90 Days</option>
        <option value="120d">120 Days</option>
      </select>
      <select v-model="window" @change="fetchData">
        <option :value="20">20 Period</option>
        <option :value="30">30 Period</option>
        <option :value="60">60 Period</option>
        <option :value="90">90 Period</option>
      </select>
      <select v-model="timeframe" @change="fetchData">
        <option value="15m">15 Minutes</option>
        <option value="1h">1 Hour</option>
        <option value="4h">4 Hours</option>
        <option value="1d">1 Day</option>
      </select>
      <button @click="fetchData" class="update-btn" :disabled="loading">
        {{ loading ? 'Loading...' : 'Update' }}
      </button>
    </div>
  </div>

  <div v-if="error" class="error-message">
    {{ error }}
  </div>

  <div class="dashboard-grid">
    <!-- Correlation Time Series -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Correlation Time Series vs {{ indexCoin }}</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <TimeSeriesChart
            v-if="timeSeriesData.correlations && timeSeriesData.correlations.length > 0"
            :series="timeSeriesData.correlations"
            y-label="Correlation"
            :y-min="-1"
            :y-max="1"
            :height="300"
          />
          <div v-else-if="!loading" class="no-data">
            No correlation data available
          </div>
        </div>
        <div class="stats-panel" v-if="stats.meanCorr !== 0">
          <div class="stat-item">
            <span class="label">Current Mean</span>
            <span class="value">{{ formatNumber(stats.meanCorr) }}</span>
          </div>
          <div class="stat-item">
            <span class="label">Current Median</span>
            <span class="value">{{ formatNumber(stats.medianCorr) }}</span>
          </div>
          <div class="stat-item">
            <span class="label">Std Dev</span>
            <span class="value">{{ formatNumber(stats.stdCorr) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Beta Time Series -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Beta Time Series vs {{ indexCoin }}</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <TimeSeriesChart
            v-if="timeSeriesData.betas && timeSeriesData.betas.length > 0"
            :series="timeSeriesData.betas"
            y-label="Beta"
            :y-min="0"
            :y-max="3"
            :height="300"
          />
          <div v-else-if="!loading" class="no-data">
            No beta data available
          </div>
        </div>
        <div class="stats-panel" v-if="stats.meanBeta !== 0">
          <div class="stat-item">
            <span class="label">Current Mean</span>
            <span class="value">{{ formatNumber(stats.meanBeta) }}</span>
          </div>
          <div class="stat-item">
            <span class="label">Current Median</span>
            <span class="value">{{ formatNumber(stats.medianBeta) }}</span>
          </div>
          <div class="stat-item">
            <span class="label">&gt; 1.0</span>
            <span class="value">{{ stats.highBetaCount }} coins</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Current Correlation Matrix -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Current Correlation Matrix</h3>
      </div>
      <div class="card-content">
        <div class="matrix-container">
          <HeatmapChart
            v-if="correlationMatrix && correlationMatrix.data.length > 0"
            :data="correlationMatrix"
            :min="-1"
            :max="1"
            color-scheme="correlation"
            :show-legend="true"
          />
          <div v-else-if="!loading" class="no-data">
            No correlation matrix data available
          </div>
        </div>
      </div>
    </div>

      <!-- Current Covariance Matrix -->
      <div class="metric-card">
        <div class="card-header">
          <h3>Current Covariance Matrix</h3>
        </div>
        <div class="card-content">
          <div class="matrix-container">
            <HeatmapChart
              v-if="covarianceMatrix && covarianceMatrix.data.length > 0"
              :data="covarianceMatrix"
              :min="0"
              :max="covarianceStats.maxCov"
              color-scheme="covariance"
              :show-legend="true"
            />
            <div v-else-if="!loading" class="no-data">
              No covariance matrix data available
            </div>
          </div>
        </div>
      </div>





    <!-- Top Correlations Table -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Current Rankings: Highest & Lowest Correlations with {{ indexCoin }}</h3>
      </div>
      <div class="card-content">
        <div class="correlations-table" v-if="correlationRankings.top.length > 0">
          <div class="table-section">
            <h4>Highest Correlations</h4>
            <div class="correlation-list">
              <div v-for="item in correlationRankings.top" :key="item.symbol" class="corr-row">
                <span class="rank">{{ item.rank }}</span>
                <span class="symbol">{{ item.symbol }}</span>
                <span class="correlation positive">{{ formatCorrelation(item.correlation) }}</span>
                <span class="beta">β={{ formatBeta(item.beta) }}</span>
                <div class="bar-container">
                  <div class="bar" :style="{ width: item.correlation * 100 + '%' }"></div>
                </div>
              </div>
            </div>
          </div>
          
          <div class="table-section">
            <h4>Lowest Correlations</h4>
            <div class="correlation-list">
              <div v-for="item in correlationRankings.bottom" :key="item.symbol" class="corr-row">
                <span class="rank">{{ item.rank }}</span>
                <span class="symbol">{{ item.symbol }}</span>
                <span class="correlation" :class="item.correlation < 0 ? 'negative' : ''">
                  {{ formatCorrelation(item.correlation) }}
                </span>
                <span class="beta">β={{ formatBeta(item.beta) }}</span>
                <div class="bar-container">
                  <div 
                    class="bar" 
                    :class="item.correlation < 0 ? 'negative' : ''"
                    :style="{ width: Math.abs(item.correlation) * 100 + '%' }"
                  ></div>
                </div>
              </div>
            </div>
          </div>
        </div>
        <div v-else-if="!loading" class="no-data">
          No ranking data available
        </div>
      </div>
    </div>

    <!-- Beta Distribution -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Beta Distribution vs {{ indexCoin }}</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <HistogramChart
            v-if="betaHistogram"
            :data="betaHistogram"
            :current-value="stats.meanBeta"
            :highlights="betaHighlights"
          />
          <div v-else-if="!loading" class="no-data">
            No beta distribution available
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useCrossAssetMatrix } from '@/composables/useCrossAssetMatrix'
import MetricCard from '../components/common/MetricCard.vue'
import HeatmapChart from '../components/charts/HeatmapChart.vue'
import HistogramChart from '../components/charts/HistogramChart.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'

// State
const indexCoin = ref('BTC')
const topN = ref(30)
const period = ref('120d')
const window = ref(120)
const timeframe = ref('1h')

// Use composable
const { 
  loading, 
  error, 
  data, 
  timeSeriesData,
  correlationRankings,
  betaHighlights,
  fullCorrelationMatrix,
  fullBetaMatrix,
  fullCovarianceMatrix, // Add this
  covarianceStats, // Add this
  stats,
  load 
} = useCrossAssetMatrix()

// Computed properties
const correlationMatrix = computed(() => {
  return fullCorrelationMatrix.value
})

const betaMatrix = computed(() => {
  return fullBetaMatrix.value
})

const covarianceMatrix = computed(() => {
  return fullCovarianceMatrix.value
})

function formatCovariance(value: number): string {
  if (Math.abs(value) < 0.001) {
    return value.toExponential(2)
  }
  return value.toFixed(6)
}


const betaHistogram = computed(() => {
  if (!data.value || !data.value.betaHistogram) return null
  return data.value.betaHistogram
})

function formatNumber(value: number): string {
  return value.toFixed(3)
}

function formatCorrelation(value: number): string {
  return value.toFixed(3)
}

function formatBeta(value: number): string {
  return value.toFixed(2)
}

// Data fetching
async function fetchData() {
  await load({
    index: indexCoin.value,
    topN: topN.value,
    period: period.value,
    window: window.value,
    timeframe: timeframe.value
  })
}

// Lifecycle
onMounted(() => {
  fetchData()
})
</script>