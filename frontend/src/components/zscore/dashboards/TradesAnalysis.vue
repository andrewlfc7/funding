<template>
  <div class="dashboard-header">
    <h2>Trades Analysis</h2>
    <div class="header-controls">
      <select v-model="selectedAssets" multiple class="multi-select">
        <option v-for="asset in availableAssets" :key="asset" :value="asset">
          {{ asset }}
        </option>
      </select>
      <select v-model="interval" @change="fetchData">
        <option value="1m">1 Min</option>
        <option value="5m">5 Min</option>
        <option value="15m">15 Min</option>
        <option value="1h">1 Hour</option>
        <option value="4h">4 Hours</option>
      </select>
      <select v-model="period" @change="fetchData">
        <option value="1h">Last Hour</option>
        <option value="24h">24 Hours</option>
        <option value="7d">7 Days</option>
      </select>
      <button @click="fetchData" class="update-btn" :disabled="loading">
        {{ loading ? 'Loading...' : 'Update' }}
      </button>
    </div>
  </div>

  <div v-if="!loading" class="dashboard-grid">
    <!-- 1. Trade Imbalance Scatter -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Trade Imbalance Analysis</h3>
        <span class="info-tooltip" title="Buy/Sell ratio vs volume relationship">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <ScatterChart
            v-if="imbalanceScatter.length > 0"
            :data="imbalanceScatter"
            x-field="buyRatio"
            y-field="volume"
            size-field="avgTradeSize"
            color-field="imbalance"
            label-field="symbol"
            x-label="Buy/Sell Ratio (%)"
            y-label="Total Volume (24h USD)"
            :show-labels="true"
            :color-scale="'diverging'"
          />
        </div>
      </div>
    </div>

    <!-- 2. Trade Size Distribution -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Trade Size Distribution</h3>
        <select v-model="selectedDistAsset" class="asset-select">
          <option v-for="asset in selectedAssets" :key="asset" :value="asset">
            {{ asset }}
          </option>
        </select>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <HistogramChart
            v-if="sizeDistribution"
            :data="sizeDistribution"
            :show-normal-overlay="true"
            x-label="Trade Size (USD)"
            y-label="Frequency"
          />
        </div>
      </div>
    </div>

    <!-- 3. Notional Value Rankings -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Average Notional Value</h3>
        <div class="interval-toggle">
          <button 
            v-for="int in ['1m', '5m', '15m', '1h']" 
            :key="int"
            @click="notionalInterval = int"
            :class="['interval-btn', { active: notionalInterval === int }]"
          >
            {{ int }}
          </button>
        </div>
      </div>
      <div class="card-content">
        <div class="notional-rankings">
          <div v-for="(item, idx) in notionalRankings" :key="item.symbol" class="notional-item">
            <span class="rank">{{ idx + 1 }}</span>
            <span class="symbol">{{ item.symbol }}</span>
            <span class="avg-size">${{ formatNumber(item.avgSize) }}</span>
            <div class="size-bar">
              <div 
                class="size-fill" 
                :style="{ width: `${(item.avgSize / maxNotional) * 100}%` }"
              ></div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 4. Trade Imbalance Time Series -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Trade Imbalance Time Series</h3>
        <span class="info-tooltip" title="Buy/Sell imbalance over time with Z-score highlights">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <TimeSeriesChart
            v-if="imbalanceTimeSeries.length > 0"
            :series="imbalanceTimeSeries"
            :y-label="'Buy/Sell Imbalance (%)'"
            :show-threshold-lines="true"
            :thresholds="[-2, -1, 0, 1, 2]"
            :threshold-labels="['-2σ', '-1σ', 'Neutral', '+1σ', '+2σ']"
            :height="300"
          />
        </div>
      </div>
    </div>

    <!-- 5. Trade Velocity Heatmap -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Trade Velocity Heatmap</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <HeatmapChart
            v-if="velocityHeatmap"
            :data="velocityHeatmap"
            :x-labels="intervalLabels"
            :y-labels="selectedAssets"
            :color-scale="'velocity'"
            :show-values="true"
            title="Trades per Minute"
          />
        </div>
      </div>
    </div>

    <!-- 6. Large Trade Detection -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Whale Trade Alerts</h3>
        <span class="whale-threshold">›99th percentile</span>
      </div>
      <div class="card-content">
        <div class="whale-trades">
          <div v-if="whaleTrades.length === 0" class="no-whales">
            No whale trades detected
          </div>
          <div v-for="trade in whaleTrades" :key="trade.id" 
               class="whale-item" :class="trade.side">
            <div class="whale-header">
              <span class="whale-symbol">{{ trade.symbol }}</span>
              <span class="whale-time">{{ formatTime(trade.timestamp) }}</span>
            </div>
            <div class="whale-details">
              <span class="whale-size">${{ formatNumber(trade.size) }}</span>
              <span class="whale-side">{{ trade.side.toUpperCase() }}</span>
              <span class="whale-price">@ ${{ trade.price.toFixed(2) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 7. Cross-Asset Trade Correlation -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Trade Pattern Correlation Matrix</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <HeatmapChart
            v-if="correlationMatrix"
            :data="correlationMatrix"
            :x-labels="selectedAssets"
            :y-labels="selectedAssets"
            :color-scale="'correlation'"
            :show-values="true"
            :symmetric="true"
          />
        </div>
      </div>
    </div>

    <!-- 8. Trade Z-Score Analysis -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Trade Z-Score Analysis</h3>
        <span class="info-tooltip" title="Current vs 30-day average">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="zscore-analysis">
          <div v-for="metric in tradeZScores" :key="metric.name" class="zscore-item">
            <div class="zscore-header">
              <span class="metric-name">{{ metric.name }}</span>
              <span class="zscore-value" :class="getZScoreClass(metric.zscore)">
                {{ metric.zscore > 0 ? '+' : ''}}{{ metric.zscore.toFixed(2) }}σ
              </span>
            </div>
            <div class="zscore-bar">
              <div class="zscore-baseline"></div>
              <div 
                class="zscore-indicator" 
                :style="{ left: `${getZScorePosition(metric.zscore)}%` }"
                :class="getZScoreClass(metric.zscore)"
              ></div>
            </div>
            <div class="zscore-context">
              <span>Now: {{ formatMetricValue(metric.current, metric.type) }}</span>
              <span>Avg: {{ formatMetricValue(metric.average, metric.type) }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 9. Trade Microstructure Stats -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Microstructure Statistics</h3>
      </div>
      <div class="card-content">
        <div class="microstructure-stats">
          <div v-for="stat in microstructureStats" :key="stat.name" class="stat-item">
            <span class="stat-name">{{ stat.name }}</span>
            <span class="stat-value">{{ stat.value }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useTradesAnalysis } from '@/composables/useTradesAnalysis'
import ScatterChart from '../components/charts/ScatterChart.vue'
import HistogramChart from '../components/charts/HistogramChart.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'
import HeatmapChart from '../components/charts/HeatmapChart.vue'
import { formatNumber } from '@/utils/formatters'

// Controls
const selectedAssets = ref(['BTC', 'ETH', 'SOL', 'AVAX', 'MATIC'])
const interval = ref('5m')
const period = ref('24h')
const selectedDistAsset = ref('BTC')
const notionalInterval = ref('5m')

// Available assets
const availableAssets = ['BTC', 'ETH', 'SOL', 'AVAX', 'MATIC', 'DOT', 'LINK', 'UNI', 'AAVE', 'SUSHI']

// Use composable
const {
  loading,
  imbalanceScatter,
  sizeDistribution,
  notionalRankings,
  imbalanceTimeSeries,
  velocityHeatmap,
  whaleTrades,
  correlationMatrix,
  tradeZScores,
  microstructureStats,
  fetchData: fetch
} = useTradesAnalysis()

// Computed
const maxNotional = computed(() => 
  Math.max(...(notionalRankings.value?.map(n => n.avgSize) || [1]))
)

const intervalLabels = computed(() => {
  // Generate labels based on interval
  return ['00:00', '04:00', '08:00', '12:00', '16:00', '20:00']
})

// Helper functions
function getZScoreClass(zscore: number) {
  const abs = Math.abs(zscore)
  if (abs >= 2) return 'extreme'
  if (abs >= 1) return 'elevated'
  return 'normal'
}

function getZScorePosition(zscore: number) {
  // Map z-score (-3 to +3) to percentage (0 to 100)
  const clamped = Math.max(-3, Math.min(3, zscore))
  return ((clamped + 3) / 6) * 100
}

function formatTime(timestamp: number) {
  return new Date(timestamp).toLocaleTimeString('en-US', {
    hour: '2-digit',
    minute: '2-digit'
  })
}

function formatMetricValue(value: number, type: string) {
  switch (type) {
    case 'count':
      return value.toFixed(0)
    case 'size':
      return '$' + formatNumber(value)
    case 'percent':
      return value.toFixed(1) + '%'
    default:
      return value.toFixed(2)
  }
}

async function fetchData() {
  await fetch({
    assets: selectedAssets.value,
    interval: interval.value,
    period: period.value
  })
}

// Watch for distribution asset change
watch(selectedDistAsset, async () => {
  // Fetch distribution
    // Fetch distribution data for selected asset
  await fetch({
    assets: selectedAssets.value,
    interval: interval.value,
    period: period.value,
    distributionAsset: selectedDistAsset.value
  })
})

// Watch for notional interval change
watch(notionalInterval, async () => {
  // Update notional rankings for selected interval
  await fetch({
    assets: selectedAssets.value,
    interval: notionalInterval.value,
    period: period.value
  })
})

onMounted(() => {
  fetchData()
})
</script>