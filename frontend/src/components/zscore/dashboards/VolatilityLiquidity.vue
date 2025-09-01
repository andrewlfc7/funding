<template>
  <div class="volatility-liquidity">
    <div class="dashboard-header">
      <h2>Volatility & Liquidity Metrics</h2>
      <div class="header-controls">
        <select v-model="selectedCoin" @change="fetchData">
          <option value="BTC">BTC</option>
          <option value="ETH">ETH</option>
          <option value="SOL">SOL</option>
          <option value="AVAX">AVAX</option>
          <option value="MATIC">MATIC</option>
        </select>
        <select v-model="period" @change="fetchData">
          <option value="7d">7 Days</option>
          <option value="30d">30 Days</option>
          <option value="90d">90 Days</option>
        </select>
        <button @click="fetchData" class="update-btn">Update</button>
      </div>
    </div>

    <div class="dashboard-grid">
      <!-- Rolling Volatility Z-Score -->
      <MetricCard 
        title="Rolling Volatility Z-Score"
        class="full-width"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="vol-zscore-container">
          <TimeSeriesChart
            v-if="volZScoreData.length > 0"
            :data="volZScoreTimeSeries"
            y-field="volZScore"
            :show-threshold-lines="true"
            :thresholds="[-2, -1, 0, 1, 2]"
            :threshold-labels="['-2σ', '-1σ', 'Mean', '+1σ', '+2σ']"
          />
          <div class="current-stats">
            <div class="stat-badge" :class="currentVolClass">
              <span class="label">Current Vol Z-Score</span>
              <span class="value">{{ currentVolZScore.toFixed(2) }}σ</span>
            </div>
            <div class="stat-badge">
              <span class="label">30D Avg</span>
              <span class="value">{{ avgVolatility.toFixed(1) }}%</span>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Daily Range Distribution -->
      <MetricCard 
        title="Daily Range Distribution"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="range-analysis">
          <HistogramChart
            v-if="rangeDistribution"
            :data="rangeDistribution"
            :current-value="currentRange"
          />
          <div class="range-stats">
            <div class="percentile-indicator">
              <span class="label">Current Range Percentile</span>
              <div class="percentile-bar">
                <div class="percentile-fill" :style="{ width: rangePercentile + '%' }"></div>
                <span class="percentile-text">{{ rangePercentile }}th</span>
              </div>
            </div>
            <div class="range-bands">
              <div class="band" v-for="band in rangeBands" :key="band.label">
                <span class="band-label">{{ band.label }}</span>
                <span class="band-value">{{ band.value.toFixed(2) }}%</span>
              </div>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Volume Distribution -->
      <MetricCard 
        title="Volume Z-Score Distribution"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="volume-analysis">
          <HistogramChart
            v-if="volumeDistribution"
            :data="volumeDistribution"
            :current-value="currentVolumeZScore"
          />
          <div class="volume-indicators">
            <div class="indicator" :class="{ active: currentVolumeZScore > 2 }">
              <span class="icon">🔥</span>
              <span class="text">High Volume</span>
            </div>
            <div class="indicator" :class="{ active: currentVolumeZScore < -1 }">
              <span class="icon">🧊</span>
              <span class="text">Low Volume</span>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Volume Z-Score vs Returns Scatter -->
      <MetricCard 
        title="Volume Z-Score vs Daily Returns"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="scatter-container">
          <ScatterChart
            v-if="scatterData.length > 0"
            :data="volZScoreVsReturns"
            x-field="returns"
            y-field="volumeZScore"
            size-field="range"
            color-field="volatility"
            x-label="Daily Returns (%)"
            y-label="Volume Z-Score"
            :show-quadrants="true"
          />
          <div class="quadrant-labels">
            <span class="q1">High Vol + Gains</span>
            <span class="q2">Low Vol + Gains</span>
            <span class="q3">Low Vol + Losses</span>
            <span class="q4">High Vol + Losses</span>
          </div>
        </div>
      </MetricCard>

      <!-- Liquidity Depth Analysis -->
      <MetricCard 
        title="Liquidity Depth Over Time"
        class="full-width"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="liquidity-depth">
          <div class="depth-chart">
            <TimeSeriesChart
              v-if="liquidityData.length > 0"
              :data="liquidityTimeSeries"
              y-field="depth"
              :secondary-y-field="'spread'"
              label="Liquidity Depth (USD)"
              secondary-label="Bid-Ask Spread (%)"
            />
          </div>
          <div class="liquidity-metrics">
            <div class="metric-grid">
              <div class="metric">
                <span class="label">Avg Daily Volume</span>
                <span class="value">${formatVolume(avgDailyVolume)}</span>
              </div>
              <div class="metric">
                <span class="label">Volume/MCap Ratio</span>
                <span class="value">{{ (volumeMcapRatio * 100).toFixed(2) }}%</span>
              </div>
              <div class="metric">
                <span class="label">Avg Spread</span>
                <span class="value">{{ avgSpread.toFixed(3) }}%</span>
              </div>
              <div class="metric">
                <span class="label">Liquidity Score</span>
                <span class="value" :class="liquidityScoreClass">{{ liquidityScore }}/10</span>
              </div>
            </div>
          </div>
        </div>
      </MetricCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import MetricCard from '../components/common/MetricCard.vue'
import HistogramChart from '../components/charts/HistogramChart.vue'
import ScatterChart from '../components/charts/ScatterChart.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'

// State
const selectedCoin = ref('BTC')
const period = ref('30d')
const loading = ref(false)
const error = ref<string | null>(null)

// Data
const volZScoreData = ref<any[]>([])
const scatterData = ref<any[]>([])
const liquidityData = ref<any[]>([])
const rangeDistribution = ref<any>(null)
const volumeDistribution = ref<any>(null)

// Computed values
const currentVolZScore = computed(() => {
  if (volZScoreData.value.length === 0) return 0
  return volZScoreData.value[volZScoreData.value.length - 1].volZScore
})

const currentVolClass = computed(() => {
  const z = currentVolZScore.value
  if (z > 2) return 'extreme-high'
  if (z > 1) return 'high'
  if (z < -2) return 'extreme-low'
  if (z < -1) return 'low'
  return 'normal'
})

const currentRange = computed(() => 4.2) // Mock
const rangePercentile = computed(() => 73) // Mock
const currentVolumeZScore = computed(() => 0.8) // Mock
const avgVolatility = computed(() => 42.5) // Mock
const avgDailyVolume = computed(() => 1250000000) // Mock
const volumeMcapRatio = computed(() => 0.045) // Mock
const avgSpread = computed(() => 0.012) // Mock
const liquidityScore = computed(() => 8.5) // Mock

const liquidityScoreClass = computed(() => {
  if (liquidityScore.value >= 8) return 'excellent'
  if (liquidityScore.value >= 6) return 'good'
  if (liquidityScore.value >= 4) return 'fair'
  return 'poor'
})

const rangeBands = computed(() => [
  { label: 'P10', value: 1.2 },
  { label: 'P25', value: 2.1 },
  { label: 'P50', value: 3.5 },
  { label: 'P75', value: 5.2 },
  { label: 'P90', value: 7.8 }
])

const volZScoreTimeSeries = computed(() => 
  volZScoreData.value.map(d => ({
    timestamp: d.timestamp,
    volZScore: d.volZScore
  }))
)

const volZScoreVsReturns = computed(() => 
  scatterData.value.map(d => ({
    returns: d.returns * 100,
    volumeZScore: d.volumeZScore,
    range: d.range,
    volatility: d.volatility
  }))
)

const liquidityTimeSeries = computed(() => 
  liquidityData.value.map(d => ({
    timestamp: d.timestamp,
    depth: d.depth,
    spread: d.spread * 100
  }))
)

// Helper functions
function formatVolume(volume: number): string {
  if (volume >= 1e9) return (volume / 1e9).toFixed(2) + 'B'
  if (volume >= 1e6) return (volume / 1e6).toFixed(2) + 'M'
  return volume.toLocaleString()
}

async function fetchData() {
  loading.value = true
  error.value = null
  
  try {
    // Mock data generation
    await new Promise(resolve => setTimeout(resolve, 500))
    
    // Generate volatility z-score time series
    const now = Date.now()
    volZScoreData.value = Array.from({ length: 100 }, (_, i) => ({
      timestamp: now - (100 - i) * 3600000,
      volZScore: Math.sin(i / 10) * 2 + (Math.random() - 0.5) * 0.5
    }))
    
    // Generate scatter data
    scatterData.value = Array.from({ length: 50 }, () => ({
      returns: (Math.random() - 0.5) * 0.1,
      volumeZScore: (Math.random() - 0.5) * 4,
      range: Math.random() * 0.1,
      volatility: 20 + Math.random() * 40
    }))
    
    // Generate liquidity data
    liquidityData.value = Array.from({ length: 100 }, (_, i) => ({
      timestamp: now - (100 - i) * 3600000,
      depth: 5000000 + Math.sin(i / 20) * 2000000 + Math.random() * 500000,
      spread: 0.01 + Math.sin(i / 15) * 0.005 + Math.random() * 0.002
    }))
    
    // Generate distributions
    rangeDistribution.value = {
      buckets: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
      counts: [2, 5, 12, 18, 25, 20, 10, 5, 2, 1, 0]
    }
    
    volumeDistribution.value = {
      buckets: [-3, -2, -1, 0, 1, 2, 3],
      counts: [5, 15, 25, 30, 20, 10, 5]
    }
    
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Failed to load data'
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  fetchData()
})
</script>

<style scoped>
/* Volatility Z-Score container */
.vol-zscore-container {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  height: 100%;
}

.current-stats {
  display: flex;
  gap: var(--space-md);
  padding: var(--space-md);
  background: var(--bg-primary);
  border-radius: 8px;
}

.stat-badge {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-xs);
  padding: var(--space-sm);
  border-radius: 6px;
  background: var(--bg-secondary);
  border: 1px solid var(--border-primary);
}

.stat-badge.extreme-high {
  border-color: #ef4444;
  background: rgba(239, 68, 68, 0.1);
}

.stat-badge.high {
  border-color: #f59e0b;
  background: rgba(245, 158, 11, 0.1);
}

.stat-badge.low {
  border-color: #3b82f6;
  background: rgba(59, 130, 246, 0.1);
}

.stat-badge.extreme-low {
  border-color: #22c55e;
  background: rgba(34, 197, 94, 0.1);
}

.stat-badge .label {
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: uppercase;
}

.stat-badge .value {
  font-size: 20px;
  font-weight: 700;
  color: var(--text-primary);
}

/* Range analysis */
.range-analysis {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  height: 100%;
}

.range-stats {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

.percentile-indicator {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.percentile-bar {
  position: relative;
  height: 24px;
  background: var(--bg-primary);
  border-radius: 12px;
  overflow: hidden;
}

.percentile-fill {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  background: linear-gradient(90deg, var(--color-accent), var(--color-positive));
  transition: width 0.3s ease;
}

.percentile-text {
  position: absolute;
  right: 8px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}

.range-bands {
  display: flex;
  justify-content: space-between;
  padding: var(--space-sm);
  background: var(--bg-primary);
  border-radius: 6px;
}

.band {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.band-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
}

.band-value {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
}

/* Volume analysis */
.volume-analysis {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  height: 100%;
}

.volume-indicators {
  display: flex;
  gap: var(--space-md);
  justify-content: center;
}

.indicator {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md);
  background: var(--bg-primary);
  border-radius: 20px;
  opacity: 0.5;
  transition: all 0.3s ease;
}

.indicator.active {
  opacity: 1;
  background: var(--bg-tertiary);
  border: 1px solid var(--border-secondary);
}

.indicator .icon {
  font-size: 20px;
}

.indicator .text {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}

/* Scatter container */
.scatter-container {
  position: relative;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.quadrant-labels {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  width: 90%;
  height: 90%;
  pointer-events: none;
  display: grid;
  grid-template-columns: 1fr 1fr;
  grid-template-rows: 1fr 1fr;
}

.quadrant-labels span {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 11px;
  color: var(--text-secondary);
  opacity: 0.6;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* Liquidity depth */
.liquidity-depth {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  height: 100%;
}

.depth-chart {
  flex: 1;
  min-height: 300px;
}

.liquidity-metrics {
  background: var(--bg-primary);
  border-radius: 8px;
  padding: var(--space-md);
}

.metric-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: var(--space-md);
}

.metric {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: var(--space-sm);
  padding: var(--space-sm);
  background: var(--bg-secondary);
  border-radius: 6px;
}

.metric .label {
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: uppercase;
}

.metric .value {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
}

.metric .value.excellent {
  color: var(--color-positive);
}

.metric .value.good {
  color: #22c55e;
}

.metric .value.fair {
  color: #f59e0b;
}

.metric .value.poor {
  color: var(--color-negative);
}

/* Responsive adjustments */
@media (max-width: 1200px) {
  .metric-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  
  .range-bands {
    flex-wrap: wrap;
  }
}

@media (max-width: 768px) {
  .current-stats {
    flex-direction: column;
  }
  
  .volume-indicators {
    flex-direction: column;
  }
  
  .metric-grid {
    grid-template-columns: 1fr;
  }
}
</style>