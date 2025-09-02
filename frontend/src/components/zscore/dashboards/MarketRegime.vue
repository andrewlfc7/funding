<template>
  <div class="market-regime">
    <div class="dashboard-header">
      <h2>Market Regime & Momentum Analysis</h2>
      <div class="header-controls">
        <select v-model="period" @change="fetchData">
          <option value="7d">7D</option>
          <option value="30d">30D</option>
          <option value="90d">90D</option>
        </select>
        <select v-model="topN" @change="fetchData">
          <option :value="10">Top 10</option>
          <option :value="20">Top 20</option>
          <option :value="50">Top 50</option>
        </select>
        <button @click="fetchData" class="update-btn" :disabled="loading">
          {{ loading ? 'Loading...' : 'Update' }}
        </button>
      </div>
    </div>

    <div class="dashboard-grid">
      <!-- Z-Score Momentum Heatmap -->
      <MetricCard 
        title="Z-Score Momentum Heatmap"
        subtitle="Rolling Z-Score Change"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="momentum-heatmap" v-if="heatmapData">
          <div class="heatmap-header">
            <div></div>
            <span v-for="tf in heatmapData.timeframes" :key="tf" class="timeframe">{{ tf }}</span>
          </div>
          <div 
            v-for="(row, coinIndex) in heatmapData.coins" 
            :key="row"
            class="heatmap-row"
          >
            <span class="symbol">{{ row }}</span>
            <div 
              v-for="(tf, tfIndex) in heatmapData.timeframes" 
              :key="tf"
              class="heatmap-cell"
              :class="getCellClass(heatmapData.matrix[coinIndex][tfIndex])"
              :style="{ opacity: getCellOpacity(heatmapData.matrix[coinIndex][tfIndex]) }"
            >
              {{ formatValue(heatmapData.matrix[coinIndex][tfIndex]) }}
            </div>
          </div>
          <div class="heatmap-legend">
            <span class="legend-label">-2</span>
            <div class="legend-gradient"></div>
            <span class="legend-label">+2</span>
          </div>
        </div>
      </MetricCard>

      <!-- Regime Transition Matrix -->
      <MetricCard 
        title="Regime Transition Matrix"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="transition-matrix">
          <table class="matrix-table">
            <thead>
              <tr>
                <th>From\To</th>
                <th>Bull</th>
                <th>Bear</th>
                <th>Range</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(from, fromIndex) in regimes" :key="from">
                <td class="regime-label">{{ capitalize(from) }}</td>
                <td 
                  v-for="(to, toIndex) in regimes" 
                  :key="to"
                  class="probability-cell"
                  :class="{ highlight: currentRegime === from && getTransitionProbability(fromIndex, toIndex) > 0.5 }"
                >
                  {{ (getTransitionProbability(fromIndex, toIndex) * 100).toFixed(0) }}%
                </td>
              </tr>
            </tbody>
          </table>
          <div class="regime-status">
            <span class="current">Current: {{ capitalize(currentRegime) }}</span>
            <span v-if="nextRegimeProbability.probability > 0" class="next">
              → {{ capitalize(nextRegimeProbability.regime) }}? 
              ({{ (nextRegimeProbability.probability * 100).toFixed(0) }}%)
            </span>
          </div>
        </div>
      </MetricCard>

      <!-- Z-Score Velocity -->
      <MetricCard 
        title="Z-Score Velocity"
        subtitle="Rate of Z-Score Change"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="velocity-chart">
          <div class="velocity-header">
            <span class="label">Accelerating ↗</span>
          </div>
          <div v-if="velocityData.length === 0" class="empty-state">
            <p>Velocity data not available for this period</p>
          </div>
          <TimeSeriesChart
            v-else
            :data="velocityData"
            y-field="velocity"
            :secondary-y-field="'acceleration'"
            :show-zero-line="true"
            :height="250"
          />
          <div class="velocity-scale">
            <span>+1</span>
            <span class="zero">0</span>
            <span>-1</span>
          </div>
        </div>
      </MetricCard>

      <!-- Cross-Asset Momentum Divergence -->
      <MetricCard 
        title="Cross-Asset Momentum"
        subtitle="Divergence"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="momentum-divergence">
          <div 
            v-for="group in momentumDivergence" 
            :key="group.category"
            class="divergence-group"
          >
            <h4>{{ capitalize(group.category) }}:</h4>
            <div class="coin-tags">
              <span 
                v-for="coin in group.coins" 
                :key="coin"
                class="coin-tag"
                :class="group.category"
              >
                {{ coin }}
              </span>
            </div>
            <div class="divergence-metric">
              <span class="label">Avg Z-Score:</span>
              <span class="value" :class="group.avgZScore > 0 ? 'positive' : 'negative'">
                {{ group.avgZScore > 0 ? '+' : '' }}{{ group.avgZScore.toFixed(2) }}
              </span>
            </div>
          </div>
          <div class="divergence-score">
            <span class="label">Divergence Score:</span>
            <span class="score" :class="divergenceScoreClass">
              {{ divergenceScore.toFixed(1) }}
            </span>
          </div>
        </div>
      </MetricCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useMarketRegime } from '@/composables/useMarketRegime'
import MetricCard from '../components/common/MetricCard.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'

const {
  loading,
  error,
  period,
  topN,
  currentRegime,
  nextRegimeProbability,
  momentumHeatmap,
  transitionMatrix,
  velocityData,
  momentumDivergence,
  fetchData
} = useMarketRegime()

// Constants
const regimes = ['bull', 'bear', 'range'] as const

// Get raw heatmap data from API
const heatmapData = computed(() => {
  const heatmap = momentumHeatmap.value
  if (!heatmap.length) return null
  
  // Extract timeframes from first item
  const timeframes = Object.keys(heatmap[0].timeframes)
  const coins = heatmap.map(h => h.symbol)
  const matrix = heatmap.map(h => timeframes.map(tf => h.timeframes[tf]))
  
  return { timeframes, coins, matrix }
})

// Computed properties
const divergenceScore = computed(() => 
  momentumDivergence.value[0]?.divergenceScore || 0
)

const divergenceScoreClass = computed(() => {
  const score = divergenceScore.value
  if (score > 2) return 'high'
  if (score > 1) return 'medium'
  return 'low'
})

// Helper functions
const capitalize = (str: string) => 
  str.charAt(0).toUpperCase() + str.slice(1)

const getCellClass = (value: number) => {
  if (value > 0) return 'positive'
  if (value < 0) return 'negative'
  return 'neutral'
}

const getCellOpacity = (value: number) => {
  const normalized = Math.min(Math.abs(value) / 2, 1)
  return 0.3 + (normalized * 0.7)
}

const formatValue = (value: number) => {
  if (Math.abs(value) < 0.05) return '0'
  return value > 0 ? `+${value.toFixed(1)}` : value.toFixed(1)
}

const getTransitionProbability = (fromIndex: number, toIndex: number) => {
  const transition = transitionMatrix.value.find(
    t => t.from === regimes[fromIndex] && t.to === regimes[toIndex]
  )
  return transition?.probability || 0
}

onMounted(() => {
  fetchData()
})
</script>

<style scoped>
/* Component uses external CSS file */
.empty-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 200px;
  color: var(--text-secondary);
  font-size: 14px;
}
</style>