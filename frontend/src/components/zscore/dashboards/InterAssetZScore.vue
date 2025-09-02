<template>
  <div class="inter-asset-zscore">
    <div class="dashboard-header">
      <h2>Inter-Asset Z-Score Correlation</h2>
      <div class="header-controls">
        <select v-model="period">
          <option value="24h">24 Hours</option>
          <option value="7d">7 Days</option>
          <option value="30d">30 Days</option>
          <option value="90d">90 Days</option>
        </select>
        <select v-model="timeframe">
          <option value="1h">1 Hour</option>
          <option value="4h">4 Hours</option>
          <option value="1d">1 Day</option>
        </select>
        <button @click="fetchData" class="update-btn" :disabled="loading">
          {{ loading ? 'Loading...' : 'Update' }}
        </button>
      </div>
    </div>

    <div class="dashboard-grid">
      <!-- Z-Score Correlation Matrix -->
      <MetricCard 
        title="Z-Score Correlation Matrix"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="matrix-container">
          <HeatmapChart
            v-if="zscoreCorrelationMatrix"
            :data="zscoreCorrelationMatrix"
            :min="-1"
            :max="1"
            color-scheme="correlation"
          />
          <div class="matrix-info">
            <p>Correlation between Z-scores (not prices)</p>
          </div>
        </div>
      </MetricCard>

      <!-- Z-Score Beta Matrix -->
      <MetricCard 
        title="Z-Score Beta Matrix"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="matrix-container">
          <HeatmapChart
            v-if="zscoreBetaMatrix"
            :data="zscoreBetaMatrix"
            :min="-2"
            :max="2"
            color-scheme="beta"
          />
          <div class="matrix-info">
            <p>Beta coefficients of Z-score relationships vs {{ indexCoin }}</p>
          </div>
        </div>
      </MetricCard>

      <!-- Z-Score Pair Divergence -->
      <MetricCard 
        title="Z-Score Pair Divergence"
        class="full-width"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="divergence-container">
          <div class="pair-selector">
            <select v-model="selectedPair">
              <option v-for="pair in pairOptions" :key="pair.pair" :value="pair.pair">
                {{ pair.coin1 }} / {{ pair.coin2 }}
              </option>
            </select>
          </div>
          
          <div class="divergence-chart">
            <TimeSeriesChart
              v-if="selectedDivergenceData"
              :data="divergenceTimeSeries"
              y-field="zscore1"
              :secondary-y-field="'zscore2'"
              :label="selectedPairCoins[0] + ' Z-Score'"
              :secondary-label="selectedPairCoins[1] + ' Z-Score'"
            />
          </div>
          
          <div class="divergence-stats">
            <div class="stat">
              <span class="label">Current Divergence</span>
              <span class="value" :class="divergenceClass">
                                {{ currentDivergence.toFixed(2) }}σ
              </span>
            </div>
            <div class="stat">
              <span class="label">Max Divergence ({{ period }})</span>
              <span class="value">{{ maxDivergence.toFixed(2) }}σ</span>
            </div>
            <div class="stat">
              <span class="label">Correlation</span>
              <span class="value">{{ pairCorrelation.toFixed(3) }}</span>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Top Z-Score Relationships -->
      <MetricCard 
        title="Strongest Z-Score Relationships"
        class="full-width"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="relationships-grid">
          <div class="relationship-section">
            <h4>Highest Positive Correlation</h4>
            <div class="relationship-list">
              <div 
                v-for="rel in topRelationships.positive" 
                :key="`${rel.coin1}-${rel.coin2}`" 
                class="relationship-item"
              >
                <div class="pair-info">
                  <span class="pair">{{ rel.coin1 }} / {{ rel.coin2 }}</span>
                  <span class="correlation positive">ρ = {{ rel.correlation.toFixed(3) }}</span>
                </div>
                <div class="visual-bar">
                  <div class="bar positive" :style="{ width: rel.correlation * 100 + '%' }"></div>
                </div>
                <div class="additional-stats">
                  <span>β = {{ rel.beta.toFixed(2) }}</span>
                  <span v-if="Math.abs(rel.beta) > 0.1">
                    Hedge: 1:{{ Math.abs(1/rel.beta).toFixed(2) }}
                  </span>
                </div>
              </div>
            </div>
          </div>
          
          <div class="relationship-section">
            <h4>Highest Negative Correlation</h4>
            <div class="relationship-list">
              <div 
                v-for="rel in topRelationships.negative" 
                :key="`${rel.coin1}-${rel.coin2}`" 
                class="relationship-item"
              >
                <div class="pair-info">
                  <span class="pair">{{ rel.coin1 }} / {{ rel.coin2 }}</span>
                  <span class="correlation negative">ρ = {{ rel.correlation.toFixed(3) }}</span>
                </div>
                <div class="visual-bar">
                  <div class="bar negative" :style="{ width: Math.abs(rel.correlation) * 100 + '%' }"></div>
                </div>
                <div class="additional-stats">
                  <span>β = {{ rel.beta.toFixed(2) }}</span>
                  <span v-if="Math.abs(rel.beta) > 0.1">
                    Hedge Ratio: {{ Math.abs(1/rel.beta).toFixed(2) }}
                  </span>
                </div>
              </div>
            </div>
          </div>
          
          <div class="relationship-section">
            <h4>Most Independent</h4>
            <div class="relationship-list">
              <div 
                v-for="coin in independentCoins" 
                :key="coin.symbol" 
                class="relationship-item"
              >
                <div class="pair-info">
                  <span class="pair">{{ coin.symbol }}</span>
                  <span class="correlation neutral">Avg |ρ| = {{ coin.avgCorrelation.toFixed(3) }}</span>
                </div>
                <div class="independence-score">
                  <span class="score-label">Independence Score</span>
                  <span class="score-value">{{ coin.independenceScore.toFixed(1) }}/10</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </MetricCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import MetricCard from '../components/common/MetricCard.vue'
import HeatmapChart from '../components/charts/HeatmapChart.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'
import { useInterAssetZScore } from '@/composables/useInterAssetZScore'

// Use the composable
const {
  loading,
  error,
  period,
  exchange,
  marketType,
  timeframe,
  indexCoin,
  zscoreCorrelationMatrix,
  zscoreBetaMatrix,
  divergenceData,
  availableCoins,
  topRelationships,
  independentCoins,
  fetchData
} = useInterAssetZScore()

// Local state for pair selection
const selectedPair = ref('')

// Computed properties for pair divergence
const pairOptions = computed(() => {
  return divergenceData.value.map(d => ({
    pair: d.pair,
    coin1: d.pair.split('-')[0],
    coin2: d.pair.split('-')[1]
  }))
})

const selectedPairCoins = computed(() => {
  if (!selectedPair.value) return ['', '']
  return selectedPair.value.split('-')
})

const selectedDivergenceData = computed(() => {
  if (!selectedPair.value || !divergenceData.value) return null
  return divergenceData.value.find(d => d.pair === selectedPair.value)
})

const divergenceTimeSeries = computed(() => {
  if (!selectedDivergenceData.value) return []
  return selectedDivergenceData.value.timeSeries.map(d => ({
    timestamp: d.timestamp * 1000, // Convert to milliseconds
    zscore1: d.zscore1,
    zscore2: d.zscore2,
    divergence: d.divergence
  }))
})

const currentDivergence = computed(() => {
  if (!selectedDivergenceData.value || selectedDivergenceData.value.timeSeries.length === 0) return 0
  const last = selectedDivergenceData.value.timeSeries[selectedDivergenceData.value.timeSeries.length - 1]
  return Math.abs(last.divergence)
})

const divergenceClass = computed(() => {
  const div = currentDivergence.value
  if (div > 3) return 'extreme'
  if (div > 2) return 'high'
  if (div > 1) return 'moderate'
  return 'low'
})

const maxDivergence = computed(() => {
  if (!selectedDivergenceData.value || selectedDivergenceData.value.timeSeries.length === 0) return 0
  return Math.max(...selectedDivergenceData.value.timeSeries.map(d => Math.abs(d.divergence)))
})

const pairCorrelation = computed(() => {
  if (!selectedPair.value || !zscoreCorrelationMatrix.value) return 0
  
  const [coin1, coin2] = selectedPairCoins.value
  const idx1 = zscoreCorrelationMatrix.value.labels.indexOf(coin1)
  const idx2 = zscoreCorrelationMatrix.value.labels.indexOf(coin2)
  
  if (idx1 === -1 || idx2 === -1) return 0
  return zscoreCorrelationMatrix.value.data[idx1][idx2]
})

// Set default selected pair when data loads
watch(pairOptions, (newOptions) => {
  if (newOptions.length > 0 && !selectedPair.value) {
    selectedPair.value = newOptions[0].pair
  }
})

// Load data on mount
onMounted(() => {
  fetchData()
})
</script>