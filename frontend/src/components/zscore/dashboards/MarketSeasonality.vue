<template>
  <div class="dashboard-header">
    <h2>Market Seasonality</h2>
    <div class="header-controls">
      <select v-model="exchange" @change="fetchData">
        <option value="binance">Binance</option>
        <option value="okx">OKX</option>
      </select>
      <select v-model="topN" @change="fetchData">
        <option :value="10">Top 10</option>
        <option :value="20">Top 20</option>
        <option :value="30">Top 30</option>
        <option :value="50">Top 50</option>
      </select>
      <select v-model="timeframe" @change="fetchData">
        <option value="1h">1H</option>
        <option value="4h">4H</option>
        <option value="1d">1D</option>
      </select>
      <select v-model="period" @change="fetchData">
        <option value="30d">30 Days</option>
        <option value="90d">90 Days</option>
        <option value="120d">120 Days</option>
      </select>
      <button @click="fetchData" class="update-btn" :disabled="loading">
        {{ loading ? 'Loading...' : 'Update' }}
      </button>
    </div>
  </div>

  <div v-if="error" class="error-message">
    {{ error }}
  </div>

  <div v-if="loading" class="loading-state">
    <div class="spinner"></div>
    <p>Loading market seasonality data...</p>
  </div>

  <div v-else-if="!loading && data" class="dashboard-grid">
    <!-- 1. Intraday Volatility Patterns (Full Width) -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Intraday Volatility Patterns (UTC)</h3>
        <span class="info-tooltip" title="Average volatility by hour of day">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <HeatmapChart
            v-if="intradayHeatmap.data.length > 0"
            :data="{
              labels: intradayHeatmap.xLabels,
              data: intradayHeatmap.data
            }"
            color-scheme="default"
            :show-legend="true"
          />
          <div v-else class="no-data">No intraday data available</div>
        </div>
      </div>
    </div>

    <!-- 2. Day of Week Analysis -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Day of Week Patterns</h3>
      </div>
      <div class="card-content">
        <div class="pattern-tabs">
          <button 
            v-for="metric in weekdayMetrics" 
            :key="metric"
            @click="weekdayMetric = metric" 
            :class="['tab-btn', { active: weekdayMetric === metric }]"
          >
            {{ metric.charAt(0).toUpperCase() + metric.slice(1) }}
          </button>
        </div>
        
        <div class="chart-container">
          <HeatmapChart
            v-if="weekdayHeatmapData && weekdayHeatmapData.data.length > 0"
            :data="weekdayHeatmapData"
            color-scheme="default"
            :show-legend="true"
          />
          <div v-else class="no-data">No weekday data available</div>
        </div>
      </div>
    </div>

    <!-- 3. Monthly Seasonality -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Monthly Seasonality</h3>
        <select v-model="selectedSeasonalityAsset" @change="createMonthlyChart" class="asset-select">
          <option v-for="asset in availableAssets" :key="asset" :value="asset">
            {{ asset }}
          </option>
        </select>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <canvas ref="monthlyChart"></canvas>
        </div>
      </div>
    </div>

    <!-- 4. Volume Autocorrelation (Full Width) -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Volume Autocorrelation Function</h3>
        <span class="info-tooltip" title="How predictable are volume patterns over time">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <canvas ref="correlogramChart"></canvas>
        </div>
      </div>
    </div>

    <!-- 5. Seasonality Z-Score Anomalies -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Current vs Historical Patterns</h3>
        <span class="info-tooltip" title="How unusual is current activity vs seasonal norms">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="anomaly-grid">
          <div v-for="anomaly in topAnomalies" :key="`${anomaly.symbol}-${anomaly.period}`" 
               class="anomaly-card" :class="getAnomalyClass(anomaly.zscore)">
            <div class="anomaly-header">
              <span class="anomaly-symbol">{{ anomaly.symbol }}</span>
              <span class="anomaly-zscore">{{ anomaly.zscore > 0 ? '+' : ''}}{{ anomaly.zscore.toFixed(1) }}σ</span>
            </div>
            <div class="anomaly-period">{{ anomaly.period }}</div>
            <div class="anomaly-metric">{{ anomaly.metric }}</div>
            <div class="anomaly-values">
              <div class="current-value">
                <span class="label">Now:</span>
                <span class="value">{{ formatValue(anomaly.current, anomaly.metric) }}</span>
              </div>
              <div class="historical-value">
                <span class="label">Usual:</span>
                <span class="value">{{ formatValue(anomaly.historical, anomaly.metric) }}</span>
              </div>
            </div>
            <div class="anomaly-status">{{ getAnomalyStatus(anomaly.zscore) }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 6. Volume Persistence Rankings -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Volume Persistence Rankings</h3>
      </div>
      <div class="card-content">
        <div class="persistence-rankings">
          <div class="persistence-header">
            <span>Asset</span>
            <span>1h</span>
            <span>4h</span>
            <span>24h</span>
            <span>Pattern</span>
          </div>
          <div v-for="item in volumePersistence" :key="item.symbol" class="persistence-item">
            <span class="symbol">{{ item.symbol }}</span>
            <span class="correlation" :class="getPersistenceClass(item.lag1h)">
              {{ item.lag1h.toFixed(2) }}
            </span>
            <span class="correlation" :class="getPersistenceClass(item.lag4h)">
              {{ item.lag4h.toFixed(2) }}
            </span>
            <span class="correlation" :class="getPersistenceClass(item.lag24h)">
              {{ item.lag24h.toFixed(2) }}
            </span>
            <span class="pattern-type">{{ item.pattern }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 7. Most Active Periods -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Most & Least Active Periods</h3>
      </div>
      <div class="card-content">
        <div class="activity-comparison">
          <div class="activity-section">
            <h4>Most Active</h4>
            <div class="activity-list">
              <div v-for="period in mostActivePeriods" :key="period.id" class="activity-item high">
                <div class="period-info">
                  <span class="asset">{{ period.asset }}</span>
                  <span class="time">{{ period.period }}</span>
                </div>
                <div class="metrics">
                  <span class="vol">Vol: {{ period.volatility.toFixed(1) }}%</span>
                  <span class="volume">Vol$: {{ formatNumber(period.volume) }}</span>
                </div>
              </div>
            </div>
          </div>
          
          <div class="activity-section">
            <h4>Least Active</h4>
            <div class="activity-list">
              <div v-for="period in leastActivePeriods" :key="period.id" class="activity-item low">
                <div class="period-info">
                  <span class="asset">{{ period.asset }}</span>
                  <span class="time">{{ period.period }}</span>
                </div>
                <div class="metrics">
                  <span class="vol">Vol: {{ period.volatility.toFixed(1) }}%</span>
                  <span class="volume">Vol$: {{ formatNumber(period.volume) }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 8. Pattern Summary Statistics -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Seasonality Insights</h3>
      </div>
      <div class="card-content">
        <div class="insights-grid">
          <div class="insight-card">
            <h4>Strongest Patterns</h4>
            <div class="insight-list">
              <div v-for="pattern in strongestPatterns" :key="pattern.id" class="insight-item">
                <span class="pattern-name">{{ pattern.name }}</span>
                <span class="pattern-strength">{{ pattern.strength }}% consistent</span>
              </div>
            </div>
          </div>
          
          <div class="insight-card">
            <h4>Volatility Clusters</h4>
            <div class="insight-list">
              <div v-for="cluster in volatilityClusters" :key="cluster.id" class="insight-item">
                <span class="cluster-time">{{ cluster.period }}</span>
                <span class="cluster-assets">{{ cluster.assets.join(', ') }}</span>
              </div>
            </div>
          </div>
          
          <div class="insight-card">
            <h4>Weekend Effect</h4>
            <div class="weekend-stats">
              <div class="stat-row" v-for="stat in weekendEffect" :key="stat.asset">
                <span class="asset">{{ stat.asset }}</span>
                <span class="effect" :class="stat.effect > 0 ? 'positive' : 'negative'">
                  {{ stat.effect > 0 ? '+' : '' }}{{ stat.effect.toFixed(1) }}% vs weekday
                </span>
              </div>
            </div>
          </div>
          
          <div class="insight-card">
            <h4>Time Zone Effects</h4>
            <div class="timezone-list">
              <div v-for="tz in timezoneEffects" :key="tz.zone" class="timezone-item">
                <span class="zone">{{ tz.zone }}</span>
                <span class="active-hours">{{ tz.activeHours }}</span>
                <span class="impact">{{ tz.impact }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick, onUnmounted } from 'vue'
import { useMarketSeasonality } from '@/composables/useMarketSeasonality'
import HeatmapChart from '../components/charts/HeatmapChart.vue'
import Chart from 'chart.js/auto'
import { formatNumber } from '@/utils/formatters'

// Define weekday metrics properly
type WeekdayMetric = 'volatility' | 'volume' | 'returns'
const weekdayMetrics: WeekdayMetric[] = ['volatility', 'volume', 'returns']

// Controls - Using topN instead of selectedAssets
const exchange = ref('binance')
const topN = ref(20)
const timeframe = ref('1h')
const period = ref('30d')
const weekdayMetric = ref<WeekdayMetric>('volatility')
const selectedSeasonalityAsset = ref('BTC')

// Chart refs
const monthlyChart = ref<HTMLCanvasElement>()
const correlogramChart = ref<HTMLCanvasElement>()

// Chart instances
let monthlyChartInstance: Chart | null = null
let correlogramChartInstance: Chart | null = null

// Labels
const weekdayLabels = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun']
const monthLabels = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']

// Use composable
const {
  loading,
  error,
  data,
  intradayHeatmap,
  weekdayVolatilityHeatmap,
  weekdayVolumeHeatmap,
  monthlySeasonality,
  volumeAutocorrelation,
  topAnomalies,
  volumePersistence,
  mostActivePeriods,
  leastActivePeriods,
  strongestPatterns,
  volatilityClusters,
  weekendEffect,
  timezoneEffects,
  load
} = useMarketSeasonality()

// Get available assets from the data
const availableAssets = computed(() => {
  const assets = new Set<string>()
  
  // Collect unique assets from monthlySeasonality
  monthlySeasonality.value.forEach(item => assets.add(item.asset))
  
  // Also check other data sources
  topAnomalies.value.forEach(item => assets.add(item.symbol))
  volumePersistence.value.forEach(item => assets.add(item.symbol))
  
  return Array.from(assets).sort()
})

// Update selectedSeasonalityAsset when available assets change
watch(availableAssets, (newAssets) => {
  if (newAssets.length > 0 && !newAssets.includes(selectedSeasonalityAsset.value)) {
    selectedSeasonalityAsset.value = newAssets[0]
  }
})

// Computed weekday heatmap based on selected metric
const weekdayHeatmapData = computed(() => {
  if (!data.value?.weekdayHeatmap) return null
  
  const heatmapData = data.value.weekdayHeatmap[weekdayMetric.value]
  if (!heatmapData || heatmapData.length === 0) return null
  
  // Get asset labels from data
  const assetLabels = heatmapData.map((_, index) => {
    // Try to get asset names from other data sources
    if (availableAssets.value[index]) {
      return availableAssets.value[index]
    }
    return `Asset ${index + 1}`
  })
  
  return {
    labels: weekdayLabels,
    data: heatmapData
  }
})

// Helper functions
function getAnomalyClass(zscore: number) {
  const absZ = Math.abs(zscore)
  if (absZ >= 2) return 'extreme'
  if (absZ >= 1) return 'moderate'
  return 'normal'
}

function getAnomalyStatus(zscore: number) {
  const absZ = Math.abs(zscore)
  if (absZ >= 2) return zscore > 0 ? 'Extremely high' : 'Extremely low'
  if (absZ >= 1) return zscore > 0 ? 'Above normal' : 'Below normal'
  return 'Normal'
}

function getPersistenceClass(correlation: number) {
  if (correlation >= 0.7) return 'strong'
  if (correlation >= 0.4) return 'medium'
  return 'weak'
}

function formatValue(value: number, metric: string) {
  switch (metric) {
    case 'volatility':
      return `${value.toFixed(1)}%`
    case 'volume':
      return `$${formatNumber(value)}`
    case 'trades':
      return value.toFixed(0)
    default:
      return value.toFixed(2)
  }
}

// Create monthly seasonality chart
function createMonthlyChart() {
  if (!monthlyChart.value) return
  
  const assetData = monthlySeasonality.value.find(item => item.asset === selectedSeasonalityAsset.value)
  if (!assetData) return
  
  // Destroy existing chart
  if (monthlyChartInstance) {
    monthlyChartInstance.destroy()
  }
  
  const ctx = monthlyChart.value.getContext('2d')
  if (!ctx) return

  monthlyChartInstance = new Chart(ctx, {
    type: 'bar',
    data: {
      labels: monthLabels,
      datasets: [
        {
          label: 'Average Return %',
          data: assetData.returns,
          backgroundColor: assetData.returns.map((v: number) => v >= 0 ? 'rgba(16, 185, 129, 0.6)' : 'rgba(239, 68, 68, 0.6)'),
          borderColor: assetData.returns.map((v: number) => v >= 0 ? 'rgba(16, 185, 129, 1)' : 'rgba(239, 68, 68, 1)'),
          borderWidth: 1
        },
        {
          label: 'Average Volatility %',
          data: assetData.volatility,
          type: 'line',
          borderColor: 'rgba(99, 102, 241, 1)',
          backgroundColor: 'rgba(99, 102, 241, 0.1)',
          yAxisID: 'y1',
          tension: 0.3
        }
      ]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: 'top',
          labels: {
            usePointStyle: true,
            padding: 10,
            font: { size: 12 }
          }
        }
      },
      scales: {
        y: {
          title: {
            display: true,
            text: 'Average Return %'
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          }
        },
        y1: {
          position: 'right',
          title: {
            display: true,
            text: 'Average Volatility %'
          },
          grid: {
            drawOnChartArea: false
          }
        },
        x: {
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          }
        }
      }
    }
  })
}

// Create correlogram - updated to use top assets
function createCorrelogram() {
  if (!correlogramChart.value || volumeAutocorrelation.value.length === 0) return
  
  // Destroy existing chart
  if (correlogramChartInstance) {
    correlogramChartInstance.destroy()
  }
  
  const ctx = correlogramChart.value.getContext('2d')
  if (!ctx) return

  // Use first 5 assets from volumeAutocorrelation for readability
  const datasets = volumeAutocorrelation.value
    .slice(0, 5)
    .map((item, idx) => ({
      label: item.asset,
      data: item.lags,
      borderColor: `hsl(${idx * 60}, 70%, 50%)`,
      backgroundColor: `hsla(${idx * 60}, 70%, 50%, 0.1)`,
      tension: 0.3,
      borderWidth: 2,
      pointRadius: 3,
      pointHoverRadius: 5
    }))

  correlogramChartInstance = new Chart(ctx, {
    type: 'line',
    data: {
      labels: Array.from({ length: 24 }, (_, i) => `${i + 1}h`),
      datasets
    },
        options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          position: 'top',
          labels: {
            usePointStyle: true,
            padding: 10,
            font: { size: 12 }
          }
        },
        tooltip: {
          mode: 'index',
          intersect: false
        }
      },
      scales: {
        x: {
          title: {
            display: true,
            text: 'Lag (hours)'
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          }
        },
        y: {
          title: {
            display: true,
            text: 'Autocorrelation (ρ)'
          },
          min: -0.2,
          max: 1,
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          },
          ticks: {
            callback: function(value) {
              return Number(value).toFixed(2)
            }
          }
        }
      }
    }
  })
}

// Data fetching - updated to use topN
async function fetchData() {
  await load({
    exchange: exchange.value,
    marketType: 'spot',
    period: period.value,
    timeframe: timeframe.value,
    topN: topN.value
  })
  
  // Recreate charts after data loads
  await nextTick()
  createMonthlyChart()
  createCorrelogram()
}

// Watch for changes
watch(selectedSeasonalityAsset, () => {
  createMonthlyChart()
})

watch(weekdayMetric, () => {
  // Weekday heatmap will update automatically via computed property
})

// Lifecycle
onMounted(() => {
  fetchData()
})

onUnmounted(() => {
  // Clean up chart instances
  if (monthlyChartInstance) {
    monthlyChartInstance.destroy()
  }
  if (correlogramChartInstance) {
    correlogramChartInstance.destroy()
  }
})
</script>