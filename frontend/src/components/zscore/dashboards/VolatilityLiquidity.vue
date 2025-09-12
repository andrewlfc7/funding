<template>
  <div class="volatility-liquidity">
    <div class="dashboard-header">
      <h2>Volatility & Liquidity Metrics</h2>
      <div class="header-controls">
        <select v-model="selectedCoin" @change="updateData">
          <option value="BTC">BTC</option>
          <option value="ETH">ETH</option>
  
        </select>
        <select v-model="period" @change="updateData">
          <option value="7d">7 Days</option>
          <option value="30d">30 Days</option>
          <option value="90d">90 Days</option>
          <option value="120d">120 Days</option>
        </select>
        <select v-model="exchange" @change="updateData">
          <option value="binance">Binance</option>
          <option value="okx">OKX</option>
        </select>
        <select v-model="marketType" @change="updateData">
          <option value="spot">Spot</option>
          <option value="perps">Perps</option>
        </select>
        <button @click="updateData" class="update-btn" :disabled="loading">
          {{ loading ? 'Loading...' : 'Update' }}
        </button>
      </div>
    </div>

    <div class="dashboard-grid">
      <!-- First row - full width -->
      <MetricCard 
        title="Rolling Volatility Z-Score"
        class="full-width"
        :loading="loading"
        :error="error"
        
        @retry="updateData"
      >
        <div class="vol-zscore-container">
          <div class="chart-wrapper">
            <TimeSeriesChart
              v-if="volZScoreTimeSeries.length > 0"
              :data="volZScoreTimeSeries"
              y-field="volZScore"
              :show-threshold-lines="true"
              :thresholds="[-2, -1, 0, 1, 2]"
              :threshold-labels="['-2σ', '-1σ', 'Mean', '+1σ', '+2σ']"
            />
          </div>
          <div class="current-stats">
            <div class="stat-badge" :class="currentVolClass">
              <span class="label">Current Vol Z-Score</span>
              <span class="value">{{ currentVolZScore.toFixed(2) }}σ</span>
            </div>
            <div class="stat-badge">
              <span class="label">Daily Range</span>
              <span class="value">{{ (currentDailyRange * 100).toFixed(2) }}%</span>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Second row - 3 columns -->
      <MetricCard 
        title="Volume Analysis"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="volume-metrics">
          <div class="metric">
            <span class="label">Current Volume</span>
            <span class="value">${{ formatVolume(currentVolume) }}</span>
          </div>
          <div class="metric">
            <span class="label">Avg Daily Volume</span>
            <span class="value">${{ formatVolume(avgDailyVolume) }}</span>
          </div>
          <div class="metric">
            <span class="label">Weekly Volume</span>
            <span class="value">${{ formatVolume(weeklyVolume) }}</span>
          </div>
          <div class="metric highlight">
            <span class="label">Volume Ratio</span>
            <span class="value" :class="volumeRatioClass">
              {{ (volumeRatio * 100).toFixed(1) }}%
            </span>
            <span class="sublabel">vs Daily Avg</span>
          </div>
        </div>
      </MetricCard>

      <MetricCard 
        title="Volume Distribution"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="distribution-analysis">
          <div class="chart-wrapper">
            <HistogramChart
              v-if="volumeDistribution && volumeDistribution.counts.length > 0"
              :data="volumeDistribution"
              :current-value="volumeDistribution.currentZScore || 0"
              x-label="Volume Z-Score"
              y-label="Frequency"
            />
          </div>
          <div class="distribution-stats">
            <div class="indicator" :class="{ active: (volumeDistribution?.currentZScore || 0) > 2 }">
              <span class="icon">🔥</span>
              <span class="text">High Volume</span>
            </div>
            <div class="indicator" :class="{ active: (volumeDistribution?.currentZScore || 0) < -2 }">
              <span class="icon">🧊</span>
              <span class="text">Low Volume</span>
            </div>
            <div class="indicator" :class="{ active: Math.abs(volumeDistribution?.currentZScore || 0) <= 1 }">
              <span class="icon">⚖️</span>
              <span class="text">Normal</span>
            </div>
          </div>
        </div>
      </MetricCard>

      <MetricCard 
        title="Volume Ratios Over Time"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="volume-ratios">
          <div class="chart-wrapper">
            <TimeSeriesChart
              v-if="volumeRatioTimeSeries.length > 0"
              :data="volumeRatioTimeSeries"
              y-field="currentRatio"
              :secondary-y-field="'ewmaRatio'"
              label="Current/Avg Ratio"
              secondary-label="EWMA/Avg Ratio"
              :y-format="(v) => (v * 100).toFixed(0) + '%'"
              :secondary-y-format="(v) => (v * 100).toFixed(0) + '%'"
            />
          </div>
          <div class="ratio-summary">
            <div class="ratio-item">
              <span class="label">Current Ratio</span>
              <span class="value" :class="getVolumeRatioClass(volumeRatio)">
                {{ (volumeRatio * 100).toFixed(1) }}%
              </span>
            </div>
            <div class="ratio-item">
              <span class="label">EWMA Ratio</span>
              <span class="value" :class="getVolumeRatioClass(ewmaRatio)">
                {{ (ewmaRatio * 100).toFixed(1) }}%
              </span>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Third row - 2 columns -->
      <MetricCard 
        title="Liquidity Depth Over Time (EWMA Volume)"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="liquidity-depth">
          <div class="chart-wrapper">
            <TimeSeriesChart
              v-if="liquidityTimeSeries.length > 0"
              :data="liquidityTimeSeries"
              y-field="depth"
              :secondary-y-field="'spread'"
              label="EWMA Volume (USD)"
              secondary-label="Bid-Ask Spread (%)"
              :y-format="formatVolumeForChart"
              :secondary-y-format="(v) => (v * 100).toFixed(3) + '%'"
            />
          </div>
          <div class="liquidity-metrics">
            <div class="metric-grid">
              <div class="metric">
                <span class="label">Avg Daily Volume</span>
                <span class="value">${{ formatVolume(avgDailyVolume) }}</span>
              </div>
              <div class="metric">
                <span class="label">Avg Hourly Volume</span>
                <span class="value">${{ formatVolume(avgHourlyVolume) }}</span>
              </div>
              <div class="metric">
                <span class="label">EWMA/Daily Ratio</span>
                <span class="value">{{ (ewmaRatio * 100).toFixed(2) }}%</span>
              </div>
              <div class="metric">
                <span class="label">Liquidity Score</span>
                <span class="value" :class="liquidityScoreClass">{{ liquidityScore }}/10</span>
              </div>
            </div>
          </div>
        </div>
      </MetricCard>

      <MetricCard 
        title="Spread vs Volume Analysis (7D Avg)"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="spread-volume-analysis">
          <div class="chart-wrapper">
            <ScatterChart
              v-if="spreadVolumeScatterData.length > 0"
              :data="spreadVolumeScatterData"
              x-field="volume"
              y-field="spread"
              size-field="volumeRatio"
              color-field="spreadZScore"
              x-label="Trading Volume (USD)"
              y-label="7D Avg Spread (%)"
              :show-quadrants="false"
            />
          </div>
          <div class="spread-insights">
            <div class="insight-item">
              <span class="label">Current</span>
              <span class="value" :class="spreadClass">{{ currentSpread.toFixed(3) }}%</span>
            </div>
            <div class="insight-item">
              <span class="label">24h Avg</span>
              <span class="value">{{ avgDailySpread.toFixed(3) }}%</span>
            </div>
            <div class="insight-item">
              <span class="label">7D Avg</span>
              <span class="value">{{ avg7DaySpread.toFixed(3) }}%</span>
            </div>
            <div class="insight-item">
              <span class="label">Z-Score</span>
              <span class="value" :class="spreadZScoreClass">{{ currentSpreadZScore.toFixed(2) }}σ</span>
            </div>
            <div class="insight-item">
              <span class="label">Corr</span>
              <span class="value">{{ spreadVolumeCorrelation.toFixed(3) }}</span>
            </div>
          </div>
        </div>
      </MetricCard>
    </div>
  </div>
</template>          

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useVolatilityLiquidity } from '@/composables/useVolatilityLiquidity'
import MetricCard from '../components/common/MetricCard.vue'
import HistogramChart from '../components/charts/HistogramChart.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'
import ScatterChart from '../components/charts/ScatterChart.vue'


import { formatVolumeForChart } from '@/utils/formatters'


// Composable
const {
  loading,
  error,
  volZScoreTimeSeries,
  volumeTimeSeries,
  liquidityTimeSeries,
  volumeRatioTimeSeries,
  currentVolZScore,
  currentDailyRange,
  currentVolume,
  avgDailyVolume,
  avgHourlyVolume,
  weeklyVolume,
  volumeRatio,
  ewmaRatio,
  volumeDistribution,
  spreadTimeSeries,
  spreadVolumeScatterData,
  fetchData
} = useVolatilityLiquidity()

// Component state
const selectedCoin = ref('BTC')
const period = ref('120d')
const exchange = ref('binance')
const marketType = ref('spot')





const volumeSpreadData = computed(() => {
  if (!volumeTimeSeries.value.length) return []
  
  return volumeTimeSeries.value.map(point => ({
    timestamp: point.timestamp,
    dollarVolume: point.dollarVolume,
    ewmaDollarVolume: point.ewmaDollarVolume,
    spread: Math.abs(point.dollarVolume - point.ewmaDollarVolume),
    ratioDeviation: ((point.dollarVolume - point.ewmaDollarVolume) / point.ewmaDollarVolume) * 100,
    label: new Date(point.timestamp).toLocaleString()
  }))
})




const currentSpread = computed(() => {
  if (!spreadTimeSeries.value.length) return 0
  return spreadTimeSeries.value[spreadTimeSeries.value.length - 1].spread1h * 100
})

const avgDailySpread = computed(() => {
  if (!spreadTimeSeries.value.length) return 0
  return spreadTimeSeries.value[spreadTimeSeries.value.length - 1].avg1d * 100
})

const currentSpreadZScore = computed(() => {
  if (!spreadTimeSeries.value.length) return 0
  return spreadTimeSeries.value[spreadTimeSeries.value.length - 1].avgZScore
})

const spreadClass = computed(() => {
  const spread = currentSpread.value
  if (spread < 0.3) return 'very-tight'
  if (spread < 0.5) return 'tight'
  if (spread > 1) return 'wide'
  if (spread > 2) return 'very-wide'
  return 'normal'
})

const spreadZScoreClass = computed(() => {
  const z = currentSpreadZScore.value
  if (Math.abs(z) > 2) return 'extreme'
  if (Math.abs(z) > 1) return 'high'
  return 'normal'
})

const avg7DaySpread = computed(() => {
  if (!spreadTimeSeries.value.length) return 0
  return spreadTimeSeries.value[spreadTimeSeries.value.length - 1].avg7d * 100
})

const spreadVolumeCorrelation = computed(() => {
  if (!spreadVolumeScatterData.value.length) return 0
  
  const x = spreadVolumeScatterData.value.map(d => d.volume)
  const y = spreadVolumeScatterData.value.map(d => d.spread) // Now using 7-day avg
  
  // Calculate correlation (same calculation as before)
  const n = x.length
  const sumX = x.reduce((a, b) => a + b, 0)
  const sumY = y.reduce((a, b) => a + b, 0)
  const sumXY = x.reduce((total, xi, i) => total + xi * y[i], 0)
  const sumX2 = x.reduce((total, xi) => total + xi * xi, 0)
  const sumY2 = y.reduce((total, yi) => total + yi * yi, 0)
  
  const correlation = (n * sumXY - sumX * sumY) / 
    Math.sqrt((n * sumX2 - sumX * sumX) * (n * sumY2 - sumY * sumY))
  
  return isNaN(correlation) ? 0 : correlation
})


// Computed values
const currentVolClass = computed(() => {
  const z = currentVolZScore.value
  if (z > 2) return 'extreme-high'
  if (z > 1) return 'high'
  if (z < -2) return 'extreme-low'
  if (z < -1) return 'low'
  return 'normal'
})

const volumeRatioClass = computed(() => {
  const ratio = volumeRatio.value
  if (ratio > 1.5) return 'very-high'
  if (ratio > 1) return 'high'
  if (ratio < 0.5) return 'low'
  if (ratio < 0.1) return 'very-low'
  return 'normal'
})

const liquidityScore = computed(() => {
  // Calculate liquidity score based on volume metrics
  let score = 5 // Base score
  
  // Add points for high volume
  if (volumeRatio.value > 1.2) score += 2
  else if (volumeRatio.value > 0.8) score += 1
  else if (volumeRatio.value < 0.3) score -= 2
  else if (volumeRatio.value < 0.5) score -= 1
  
  // Add points for consistent EWMA
  if (ewmaRatio.value > 0.8 && ewmaRatio.value < 1.2) score += 1
  
  // Add points for absolute volume
  if (avgDailyVolume.value > 1e9) score += 2
  else if (avgDailyVolume.value > 500e6) score += 1
  
  return Math.max(1, Math.min(10, score))
})

const liquidityScoreClass = computed(() => {
  if (liquidityScore.value >= 8) return 'excellent'
  if (liquidityScore.value >= 6) return 'good'
  if (liquidityScore.value >= 4) return 'fair'
  return 'poor'
})

// Methods
function formatVolume(volume: number): string {
  if (!volume || volume === 0) return '0'
  if (volume >= 1e9) return (volume / 1e9).toFixed(2) + 'B'
  if (volume >= 1e6) return (volume / 1e6).toFixed(2) + 'M'
  if (volume >= 1e3) return (volume / 1e3).toFixed(2) + 'K'
  return volume.toFixed(2)
}

function getVolumeRatioClass(ratio: number): string {
  if (ratio > 1.5) return 'very-high'
  if (ratio > 1) return 'high'
  if (ratio < 0.5) return 'low'
  if (ratio < 0.1) return 'very-low'
  return 'normal'
}

async function updateData() {
  await fetchData({
    coin: selectedCoin.value,
    period: period.value,
    exchange: exchange.value,
    marketType: marketType.value
  })
}


// Lifecycle
onMounted(() => {
  updateData()
})

// Auto-refresh every 30 seconds
let refreshInterval: number | null = null

onMounted(() => {
  refreshInterval = window.setInterval(() => {
    updateData()
  }, 30000)
})

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
  }
})
</script>