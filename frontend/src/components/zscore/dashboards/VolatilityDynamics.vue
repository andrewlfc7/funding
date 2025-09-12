<template>
  <div class="dashboard-header">
    <h2>Volatility Dynamics</h2>
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
        <option value="7d">7 Days</option>
        <option value="30d">30 Days</option>
        <option value="90d">90 Days</option>
      </select>
      <button @click="fetchData" class="update-btn" :disabled="loading">
        {{ loading ? 'Loading...' : 'Update' }}
      </button>
    </div>
  </div>

  <div v-if="error" class="error-message">
    {{ error }}
  </div>

  <div v-if="!loading" class="dashboard-grid">
    <!-- 1. Volatility-of-Volatility Time Series (Full Width) -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Volatility-of-Volatility (VoV) Time Series</h3>
        <span class="info-tooltip" title="Rolling standard deviation of volatility itself - identifies regime changes">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <TimeSeriesChart
            v-if="vovTimeSeries.length > 0"
            :series="vovTimeSeries"
            :y-label="'VoV (σ)'"
            :height="400"
            :show-grid="true"
            :max-series="15"
            :show-legend="true"
            :y-format="(value) => value.toFixed(2) + 'σ'"
          />

          <div v-else class="no-data">No VoV data available</div>
        </div>
      </div>
    </div>

    <!-- 2. Volatility Skewness Time Series (Full Width) -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Volatility Skewness Time Series</h3>
        <span class="info-tooltip" title="Positive = more upside vol spikes, Negative = more downside vol">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <TimeSeriesChart
            v-if="skewnessTimeSeries.length > 0"
            :series="skewnessTimeSeries"
            :y-label="'Skewness'"
            :height="300"
            :show-grid="false"

          />
          <div v-else class="no-data">No skewness data available</div>
        </div>
      </div>
    </div>

    <!-- 3. Skewness vs Kurtosis Scatter -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Volatility Shape Analysis</h3>
        <span class="info-tooltip" title="Skewness vs Kurtosis profile">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <ScatterChart
            v-if="skewKurtosisScatter.length > 0"
            :data="skewKurtosisScatter"
            x-field="skewness"
            y-field="kurtosis"
            label-field="symbol"
            x-label="Skewness"
            y-label="Excess Kurtosis"
            :show-labels="true"
            :quadrants="true"
            :quadrant-labels="quadrantLabels"
          />
          <div v-else class="no-data">No scatter data available</div>
        </div>
      </div>
    </div>

    <!-- 4. Volatility Distribution Analysis -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Volatility Distribution</h3>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <HistogramChart
            v-if="histogramData"
            :data="histogramData"
            :current-value="currentVolValue"
            :highlights="distributionHighlights"
          />
          <div v-else class="no-data">No distribution data available</div>
        </div>
      </div>
    </div>

    <!-- 5. Rolling Covariance: Vol vs Dollar Volume (Full Width) -->
    <div class="metric-card full-width">
      <div class="card-header">
        <h3>Rolling Covariance: Volatility vs Dollar Volume</h3>
        <span class="info-tooltip" title="Identifies periods where flow drives volatility">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="chart-container">
          <TimeSeriesChart
            v-if="covarianceTimeSeries.length > 0"
            :series="covarianceTimeSeries"
            :y-label="'Covariance'"
            :height="300"
          />
          <div v-else class="no-data">No covariance data available</div>
        </div>
      </div>
    </div>

    <!-- 6. Volatility Regime Classification -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Current Volatility Regime Classification</h3>
      </div>
      <div class="card-content">
        <div class="regime-grid">
          <div v-for="asset in regimeClassification" :key="asset.symbol" 
               class="regime-card" :class="asset.regime">
            <div class="regime-symbol">{{ asset.symbol }}</div>
            <div class="regime-type">{{ capitalizeFirst(asset.regime) }}</div>
            <div class="regime-metrics">
              <div class="metric">
                <span class="label">Vol:</span>
                <span class="value">{{ asset.volatility.toFixed(1) }}%</span>
              </div>
              <div class="metric">
                <span class="label">Skew:</span>
                <span class="value">{{ asset.skewness.toFixed(2) }}</span>
              </div>
              <div class="metric">
                <span class="label">Kurt:</span>
                <span class="value">{{ asset.kurtosis.toFixed(2) }}</span>
              </div>
            </div>
            <div class="regime-description">{{ getRegimeDescription(asset.regime) }}</div>
          </div>
        </div>
      </div>
    </div>

    <!-- 7. VoV & Skewness Rankings -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Volatility Instability Rankings</h3>
      </div>
      <div class="card-content">
        <div class="instability-rankings">
          <div class="ranking-header">
            <span>Asset</span>
            <span>VoV</span>
            <span>Skew</span>
            <span>Status</span>
          </div>
          <div v-for="(asset, idx) in instabilityRankings" :key="asset.symbol" class="ranking-item">
            <span class="rank">{{ idx + 1 }}</span>
            <span class="symbol">{{ asset.symbol }}</span>
            <span class="vov" :class="getVovClass(asset.vov)">{{ asset.vov.toFixed(2) }}σ</span>
            <span class="skew" :class="getSkewClass(asset.skewness)">{{ asset.skewness.toFixed(2) }}</span>
            <span class="status" :class="asset.status">{{ asset.status_label }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 8. Flow-Vol Analysis -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Flow-Volatility Beta Rankings</h3>
        <span class="info-tooltip" title="Higher beta = stronger flow-volatility link">ⓘ</span>
      </div>
      <div class="card-content">
        <div class="beta-rankings">
          <div class="beta-header">
            <span>Asset</span>
            <span>Beta</span>
            <span>R²</span>
            <span>Link</span>
          </div>
          <div v-for="(item, idx) in flowVolBeta" :key="item.symbol" class="beta-item">
            <span class="rank">{{ idx + 1 }}</span>
            <span class="symbol">{{ item.symbol }}</span>
            <span class="beta">{{ item.beta.toFixed(2) }}</span>
            <span class="r-squared">{{ (item.r_squared * 100).toFixed(0) }}%</span>
            <span class="link-strength" :class="getBetaClass(item.beta)">
              {{ getBetaLabel(item.beta) }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- 9. Volatility Statistics Summary -->
    <div class="metric-card span-2">
      <div class="card-header">
        <h3>Volatility Statistics Summary</h3>
      </div>
      <div class="card-content">
        <div class="stats-table">
          <div class="stats-header">
            <span>Asset</span>
            <span>Current Vol</span>
            <span>30d Avg</span>
            <span>Percentile</span>
            <span>Skewness</span>
            <span>Kurtosis</span>
            <span>VoV</span>
          </div>
          <div v-for="stat in volStatsSummary" :key="stat.symbol" class="stats-row">
            <span class="symbol">{{ stat.symbol }}</span>
            <span class="current-vol" :class="getVolClass(stat.current_vol, stat.avg_vol)">
              {{ stat.current_vol.toFixed(1) }}%
            </span>
            <span class="avg-vol">{{ stat.avg_vol.toFixed(1) }}%</span>
            <span class="percentile" :class="getPercentileClass(Number(stat.percentile))">
              {{ stat.percentile }}th
            </span>
            <span class="skewness" :class="getSkewClass(stat.skewness)">
              {{ stat.skewness.toFixed(2) }}
            </span>
            <span class="kurtosis" :class="getKurtosisClass(stat.kurtosis)">
              {{ stat.kurtosis.toFixed(2) }}
            </span>
            <span class="vov" :class="getVovClass(stat.vov)">
              {{ stat.vov.toFixed(2) }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>

  <div v-else-if="loading" class="loading-state">
    <div class="spinner"></div>
    <p>Loading volatility dynamics...</p>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useVolatilityDynamics } from '@/composables/useVolatilityDynamics'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'
import ScatterChart from '../components/charts/ScatterChart.vue'
import HistogramChart from '../components/charts/HistogramChart.vue'
import { useMetaData } from '@/composables/useMetaData'

// Define regime type
type RegimeType = 'normal' | 'stressed' | 'euphoric' | 'compressed' | 'unstable'



const { 
  exchanges: availableExchanges, 
  coins: availableCoins, 
  loading: metaLoading, 
  loadMetaData 
} = useMetaData()

const topN = ref(20)
const timeframe = ref('1h')
const period = ref('30d')
const selectedDistAsset = ref('')

const exchange = ref('')

// Quadrant labels for scatter plot
const quadrantLabels = {
  topRight: 'Fat Tails\n+ Right Skew',
  topLeft: 'Fat Tails\n+ Left Skew',
  bottomRight: 'Thin Tails\n+ Right Skew',
  bottomLeft: 'Thin Tails\n+ Left Skew'
}

// Use composable
const {
  loading,
  error,
  vovTimeSeries,
  skewnessTimeSeries,
  skewKurtosisScatter,
  volDistribution,
  distributionStats,
  covarianceTimeSeries,
  regimeClassification,
  instabilityRankings,
  flowVolBeta,
  volStatsSummary,
  fetchData: fetch
} = useVolatilityDynamics()


const availableAssets = computed(() => {
  const assets = new Set<string>()
  
  // Add coins from meta data
  availableCoins.value.forEach(coin => assets.add(coin))
  
  // Also add assets from the volatility data (if any)
  volStatsSummary.value.forEach(stat => assets.add(stat.symbol))
  regimeClassification.value.forEach(item => assets.add(item.symbol))
  instabilityRankings.value.forEach(item => assets.add(item.symbol))
  
  return Array.from(assets).sort()
})

watch(availableAssets, (newAssets) => {
  if (newAssets.length > 0 && (!selectedDistAsset.value || !newAssets.includes(selectedDistAsset.value))) {
    selectedDistAsset.value = newAssets[0]
  }
})



watch(availableExchanges, (newExchanges) => {
  if (newExchanges.length > 0 && !exchange.value) {
    exchange.value = newExchanges[0]
  }
})

// Computed properties for histogram
const histogramData = computed(() => {
  if (!volDistribution.value) return null
  return {
    buckets: volDistribution.value.bins,
    counts: volDistribution.value.frequencies
  }
})

const currentVolValue = computed(() => {
  const stat = volStatsSummary.value.find(s => s.symbol === selectedDistAsset.value)
  return stat ? stat.current_vol : undefined
})

const distributionHighlights = computed(() => {
  if (!distributionStats.value) return []
  return distributionStats.value
    .filter(stat => stat.line)
    .map(stat => ({
      value: stat.value,
      label: stat.label
    }))
})

// Helper functions
function capitalizeFirst(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1)
}

function getRegimeDescription(regime: RegimeType): string {
  const descriptions: Record<RegimeType, string> = {
    'normal': 'Low volatility, symmetric distribution',
    'stressed': 'High volatility, negative skew',
    'euphoric': 'High volatility, positive skew',
    'compressed': 'Low volatility, high kurtosis',
    'unstable': 'High VoV, changing regime'
  }
  return descriptions[regime] || 'Unknown regime'
}

function getVovClass(vov: number) {
  if (vov >= 3) return 'extreme'
  if (vov >= 2) return 'high'
  if (vov >= 1) return 'medium'
  return 'low'
}

function getSkewClass(skew: number) {
  if (skew > 1) return 'positive-high'
  if (skew > 0.5) return 'positive'
  if (skew < -1) return 'negative-high'
  if (skew < -0.5) return 'negative'
  return 'neutral'
}

function getKurtosisClass(kurtosis: number) {
  if (kurtosis > 3) return 'very-high'
  if (kurtosis > 1) return 'high'
  if (kurtosis < -1) return 'low'
  return 'normal'
}

function getBetaClass(beta: number) {
  if (beta >= 1.5) return 'strong'
  if (beta >= 0.8) return 'medium'
  return 'weak'
}

function getBetaLabel(beta: number) {
  if (beta >= 1.5) return 'Strong'
  if (beta >= 0.8) return 'Medium'
  return 'Weak'
}

function getVolClass(current: number, avg: number) {
  const ratio = current / avg
  if (ratio > 1.5) return 'very-high'
  if (ratio > 1.2) return 'high'
  if (ratio < 0.5) return 'very-low'
  if (ratio < 0.8) return 'low'
  return 'normal'
}

function getPercentileClass(percentile: number) {
  if (percentile >= 90) return 'extreme-high'
  if (percentile >= 75) return 'high'
  if (percentile <= 10) return 'extreme-low'
  if (percentile <= 25) return 'low'
  return 'normal'
}

// Updated data fetching to use topN
async function fetchData() {
  await fetch({
    topN: topN.value,
    timeframe: timeframe.value,
    period: period.value,
    distributionAsset: selectedDistAsset.value,
    exchange: exchange.value || 'binance',
    marketType: 'spot'
  })
}

// Watch for distribution asset change
watch(selectedDistAsset, () => {
  fetchData()
})


onMounted(async () => {
  try {
    await loadMetaData('spot', 'USDT') 
    
    // Set defaults from meta data
    if (availableExchanges.value.length > 0) {
      exchange.value = availableExchanges.value[0]
    }
    
    if (availableCoins.value.length > 0) {
      selectedDistAsset.value = availableCoins.value[0]
    }
    
    // Fetch volatility data
    await fetchData()
  } catch (error) {
    console.error('Failed to initialize:', error)
  }
})

</script>
