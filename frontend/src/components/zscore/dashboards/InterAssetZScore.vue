<template>
  <div class="inter-asset-zscore">
    <div class="dashboard-header">
      <h2>Inter-Asset Z-Score Correlation</h2>
      <div class="header-controls">
        <select v-model="period" @change="fetchData">
          <option value="24h">24 Hours</option>
          <option value="7d">7 Days</option>
          <option value="30d">30 Days</option>
        </select>
        <button @click="fetchData" class="update-btn">Update</button>
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
            <p>Beta coefficients of Z-score relationships</p>
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
            <select v-model="selectedPair.coin1">
              <option v-for="coin in topCoins" :key="coin">{{ coin }}</option>
            </select>
            <span class="vs">vs</span>
            <select v-model="selectedPair.coin2">
              <option v-for="coin in topCoins" :key="coin">{{ coin }}</option>
            </select>
          </div>
          
          <div class="divergence-chart">
            <TimeSeriesChart
              v-if="divergenceData.length > 0"
              :data="divergenceTimeSeries"
              y-field="zscore1"
              :secondary-y-field="'zscore2'"
              :label="selectedPair.coin1 + ' Z-Score'"
              :secondary-label="selectedPair.coin2 + ' Z-Score'"
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
              <span class="label">Max Divergence (30D)</span>
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
              <div v-for="rel in topPositiveRelationships" :key="rel.pair" class="relationship-item">
                <div class="pair-info">
                  <span class="pair">{{ rel.coin1 }} / {{ rel.coin2 }}</span>
                  <span class="correlation positive">ρ = {{ rel.correlation.toFixed(3) }}</span>
                </div>
                <div class="visual-bar">
                  <div class="bar positive" :style="{ width: rel.correlation * 100 + '%' }"></div>
                </div>
                <div class="additional-stats">
                  <span>β = {{ rel.beta.toFixed(2) }}</span>
                  <span>Lead: {{ rel.leadLag > 0 ? rel.coin1 : rel.coin2 }}</span>
                </div>
              </div>
            </div>
          </div>
          
          <div class="relationship-section">
            <h4>Highest Negative Correlation</h4>
            <div class="relationship-list">
              <div v-for="rel in topNegativeRelationships" :key="rel.pair" class="relationship-item">
                <div class="pair-info">
                  <span class="pair">{{ rel.coin1 }} / {{ rel.coin2 }}</span>
                  <span class="correlation negative">ρ = {{ rel.correlation.toFixed(3) }}</span>
                </div>
                <div class="visual-bar">
                  <div class="bar negative" :style="{ width: Math.abs(rel.correlation) * 100 + '%' }"></div>
                </div>
                <div class="additional-stats">
                  <span>β = {{ rel.beta.toFixed(2) }}</span>
                  <span>Hedge Ratio: {{ Math.abs(1/rel.beta).toFixed(2) }}</span>
                </div>
              </div>
            </div>
          </div>
          
          <div class="relationship-section">
            <h4>Most Independent</h4>
            <div class="relationship-list">
              <div v-for="coin in independentCoins" :key="coin.symbol" class="relationship-item">
                <div class="pair-info">
                  <span class="pair">{{ coin.symbol }}</span>
                  <span class="correlation neutral">Avg ρ = {{ coin.avgCorrelation.toFixed(3) }}</span>
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
import { ref, computed, onMounted } from 'vue'
import MetricCard from '../components/common/MetricCard.vue'
import HeatmapChart from '../components/charts/HeatmapChart.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'

// State
const period = ref('30d')
const loading = ref(false)
const error = ref<string | null>(null)

const topCoins = ['BTC', 'ETH', 'SOL', 'AVAX', 'MATIC', 'DOT', 'LINK', 'UNI', 'AAVE', 'CRV']
const selectedPair = ref({ coin1: 'BTC', coin2: 'ETH' })

// Data
const zscoreCorrelationMatrix = ref<any>(null)
const zscoreBetaMatrix = ref<any>(null)
const divergenceData = ref<any[]>([])

// Computed
const currentDivergence = computed(() => {
  if (divergenceData.value.length === 0) return 0
  const last = divergenceData.value[divergenceData.value.length - 1]
  return Math.abs(last.zscore1 - last.zscore2)
})

const divergenceClass = computed(() => {
  const div = currentDivergence.value
  if (div > 3) return 'extreme'
  if (div > 2) return 'high'
  if (div > 1) return 'moderate'
  return 'low'
})

const maxDivergence = computed(() => {
  if (divergenceData.value.length === 0) return 0
  return Math.max(...divergenceData.value.map(d => Math.abs(d.zscore1 - d.zscore2)))
})

const pairCorrelation = computed(() => 0.78) // Mock

const divergenceTimeSeries = computed(() => 
  divergenceData.value.map(d => ({
    timestamp: d.timestamp,
    zscore1: d.zscore1,
    zscore2: d.zscore2
  }))
)

const topPositiveRelationships = computed(() => [
  { pair: 'ETH/DOT', coin1: 'ETH', coin2: 'DOT', correlation: 0.92, beta: 1.15, leadLag: 2 },
  { pair: 'SOL/AVAX', coin1: 'SOL', coin2: 'AVAX', correlation: 0.89, beta: 0.98, leadLag: -1 },
  { pair: 'LINK/UNI', coin1: 'LINK', coin2: 'UNI', correlation: 0.87, beta: 1.23, leadLag: 0 }
])

const topNegativeRelationships = computed(() => [
  { pair: 'BTC/USDT', coin1: 'BTC', coin2: 'USDT', correlation: -0.82, beta: -0.05, leadLag: 0 },
  { pair: 'ETH/DAI', coin1: 'ETH', coin2: 'DAI', correlation: -0.75, beta: -0.03, leadLag: 1 },
  { pair: 'SOL/USDC', coin1: 'SOL', coin2: 'USDC', correlation: -0.68, beta: -0.02, leadLag: 0 }
])

const independentCoins = computed(() => [
  { symbol: 'LINK', avgCorrelation: 0.15, independenceScore: 8.5 },
  { symbol: 'ATOM', avgCorrelation: 0.22, independenceScore: 7.8 },
  { symbol: 'THETA', avgCorrelation: 0.28, independenceScore: 7.2 }
])

async function fetchData() {
  loading.value = true
  error.value = null
  
  try {
    await new Promise(resolve => setTimeout(resolve, 500))
    
    // Mock correlation matrix
    const matrixSize = topCoins.length
    const corrData = Array(matrixSize).fill(null).map((_, i) => 
      Array(matrixSize).fill(null).map((_, j) => {
        if (i === j) return 1
        return Math.random() * 2 - 1
      })
    )
    
    zscoreCorrelationMatrix.value = {
      labels: topCoins,
      data: corrData
    }
    
    // Mock beta matrix
    const betaData = Array(matrixSize).fill(null).map((_, i) => 
      Array(matrixSize).fill(null).map((_, j) => {
        if (i === j) return 1
        return (Math.random() - 0.5) * 3
      })
    )
    
    zscoreBetaMatrix.value = {
      labels: topCoins,
      data: betaData
    }
    
    // Mock divergence data
    const now = Date.now()
    divergenceData.value = Array.from({ length: 100 }, (_, i) => ({
      timestamp: now - (100 - i) * 3600000,
      zscore1: Math.sin(i / 10) * 2 + (Math.random() - 0.5),
      zscore2: Math.sin(i / 10 + 0.5) * 1.8 + (Math.random() - 0.5)
    }))
    
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
/* Matrix containers */
.matrix-container {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  height: 100%;
  min-height: 350px;
}

.matrix-info {
  font-size: 12px;
  color: var(--text-secondary);
  text-align: center;
  padding: var(--space-sm);
  background: var(--bg-primary);
  border-radius: 4px;
}

/* Divergence section */
.divergence-container {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

.pair-selector {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  padding: var(--space-md);
  background: var(--bg-primary);
  border-radius: 6px;
}

.pair-selector select {
  padding: 6px 12px;
  border-radius: 4px;
  border: 1px solid var(--border-primary);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-weight: 600;
}

.vs {
  font-size: 14px;
  color: var(--text-secondary);
  font-weight: 500;
}

.divergence-chart {
  height: 300px;
}

.divergence-stats {
  display: flex;
  gap: var(--space-lg);
  padding: var(--space-md);
  background: var(--bg-primary);
  border-radius: 6px;
}

.divergence-stats .stat {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-xs);
}

.divergence-stats .label {
  font-size: 12px;
  color: var(--text-secondary);
  text-transform: uppercase;
}

.divergence-stats .value {
  font-size: 20px;
  font-weight: 700;
}

.divergence-stats .value.extreme {
  color: #ef4444;
}

.divergence-stats .value.high {
  color: #f59e0b;
}

.divergence-stats .value.moderate {
  color: #3b82f6;
}

.divergence-stats .value.low {
  color: #22c55e;
}

/* Relationships grid */
.relationships-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: var(--space-lg);
}

.relationship-section h4 {
  margin: 0 0 var(--space-md) 0;
  font-size: 13px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.relationship-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.relationship-item {
  padding: var(--space-sm);
  background: var(--bg-primary);
  border-radius: 6px;
  border: 1px solid var(--border-primary);
}

.pair-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-xs);
}

.pair {
  font-weight: 600;
  color: var(--text-primary);
}

.correlation {
  font-weight: 700;
  font-family: var(--font-mono);
}

.correlation.positive {
  color: var(--color-positive);
}

.correlation.negative {
  color: var(--color-negative);
}

.correlation.neutral {
  color: var(--text-secondary);
}

.visual-bar {
  height: 8px;
  background: var(--bg-secondary);
  border-radius: 4px;
  margin: var(--space-xs) 0;
  overflow: hidden;
}

.visual-bar .bar {
  height: 100%;
  transition: width 0.3s ease;
}

.visual-bar .bar.positive {
  background: var(--color-positive);
}

.visual-bar .bar.negative {
  background: var(--color-negative);
  float: right;
}

.additional-stats {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: var(--space-xs);
}

.independence-score {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-xs);
  background: var(--bg-secondary);
  border-radius: 4px;
}

.score-label {
  font-size: 11px;
  color: var(--text-secondary);
}

.score-value {
  font-weight: 700;
  color: var(--color-accent);
}
</style>