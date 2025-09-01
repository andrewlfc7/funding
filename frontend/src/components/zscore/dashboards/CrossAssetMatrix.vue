<template>
  <div class="cross-asset-matrix">
    <div class="dashboard-header">
      <h2>Cross-Asset Matrix (BTC as Index)</h2>
      <div class="header-controls">
        <select v-model="period" @change="fetchData">
          <option value="24h">24 Hours</option>
          <option value="7d">7 Days</option>
          <option value="30d">30 Days</option>
          <option value="90d">90 Days</option>
        </select>
        <select v-model="window" @change="fetchData">
          <option :value="20">20 Period</option>
          <option :value="30">30 Period</option>
          <option :value="60">60 Period</option>
        </select>
        <button @click="fetchData" class="update-btn">Update</button>
      </div>
    </div>

    <div class="dashboard-grid">
      <!-- Correlation Matrix -->
      <MetricCard 
        title="Correlation Matrix vs BTC"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="matrix-container">
          <HeatmapChart
            v-if="correlationMatrix"
            :data="correlationMatrix"
            :min="-1"
            :max="1"
            color-scheme="correlation"
          />
          <div class="matrix-legend">
            <span class="legend-label">Negative</span>
            <div class="legend-gradient correlation"></div>
            <span class="legend-label">Positive</span>
          </div>
        </div>
      </MetricCard>

      <!-- Beta Matrix -->
      <MetricCard 
        title="Beta Matrix vs BTC"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="matrix-container">
          <HeatmapChart
            v-if="betaMatrix"
            :data="betaMatrix"
            :min="0"
            :max="2"
            color-scheme="beta"
          />
          <div class="matrix-legend">
            <span class="legend-label">Low Beta</span>
            <div class="legend-gradient beta"></div>
            <span class="legend-label">High Beta</span>
          </div>
        </div>
      </MetricCard>

      <!-- Correlation Distribution -->
      <MetricCard 
        title="Correlation Distribution"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="distribution-container">
          <HistogramChart
            v-if="corrDistribution"
            :data="corrDistribution"
          />
          <div class="stats-panel">
            <div class="stat-item">
              <span class="label">Mean</span>
              <span class="value">{{ stats.meanCorr.toFixed(3) }}</span>
            </div>
            <div class="stat-item">
              <span class="label">Median</span>
              <span class="value">{{ stats.medianCorr.toFixed(3) }}</span>
            </div>
            <div class="stat-item">
              <span class="label">Std Dev</span>
              <span class="value">{{ stats.stdCorr.toFixed(3) }}</span>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Beta Distribution -->
      <MetricCard 
        title="Beta Distribution"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="distribution-container">
          <HistogramChart
            v-if="betaDistribution"
            :data="betaDistribution"
          />
          <div class="stats-panel">
            <div class="stat-item">
              <span class="label">Mean</span>
              <span class="value">{{ stats.meanBeta.toFixed(3) }}</span>
            </div>
            <div class="stat-item">
              <span class="label">Median</span>
              <span class="value">{{ stats.medianBeta.toFixed(3) }}</span>
            </div>
            <div class="stat-item">
              <span class="label">&gt; 1.0</span>
              <span class="value">{{ stats.highBetaCount }} coins</span>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Top Correlations Table -->
      <MetricCard 
        title="Highest & Lowest Correlations with BTC"
        class="full-width"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="correlations-table">
          <div class="table-section">
            <h4>Highest Correlations</h4>
            <div class="correlation-list">
              <div v-for="item in topCorrelations" :key="item.symbol" class="corr-row">
                <span class="rank">{{ item.rank }}</span>
                <span class="symbol">{{ item.symbol }}</span>
                <span class="correlation positive">{{ item.correlation.toFixed(3) }}</span>
                <span class="beta">β={{ item.beta.toFixed(2) }}</span>
                <div class="bar-container">
                  <div class="bar" :style="{ width: item.correlation * 100 + '%' }"></div>
                </div>
              </div>
            </div>
          </div>
          
          <div class="table-section">
            <h4>Lowest Correlations</h4>
            <div class="correlation-list">
              <div v-for="item in bottomCorrelations" :key="item.symbol" class="corr-row">
                <span class="rank">{{ item.rank }}</span>
                <span class="symbol">{{ item.symbol }}</span>
                <span class="correlation negative">{{ item.correlation.toFixed(3) }}</span>
                <span class="beta">β={{ item.beta.toFixed(2) }}</span>
                <div class="bar-container">
                  <div class="bar negative" :style="{ width: Math.abs(item.correlation) * 100 + '%' }"></div>
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
import HistogramChart from '../components/charts/HistogramChart.vue'

// State
const period = ref('30d')
const window = ref(30)
const loading = ref(false)
const error = ref<string | null>(null)

// Data
const correlationMatrix = ref<any>(null)
const betaMatrix = ref<any>(null)
const corrDistribution = ref<any>(null)
const betaDistribution = ref<any>(null)

// Mock data
const coins = ['ETH', 'SOL', 'AVAX', 'MATIC', 'DOT', 'LINK', 'UNI', 'AAVE', 'CRV', 'FTM']

const stats = ref({
  meanCorr: 0.67,
  medianCorr: 0.72,
  stdCorr: 0.18,
  meanBeta: 1.15,
  medianBeta: 1.08,
  highBetaCount: 12
})

const topCorrelations = computed(() => {
  // Mock data - replace with real data
  return [
    { rank: 1, symbol: 'ETH', correlation: 0.92, beta: 1.12 },
    { rank: 2, symbol: 'LINK', correlation: 0.89, beta: 1.35 },
    { rank: 3, symbol: 'DOT', correlation: 0.87, beta: 1.28 },
    { rank: 4, symbol: 'UNI', correlation: 0.85, beta: 1.42 },
    { rank: 5, symbol: 'AAVE', correlation: 0.83, beta: 1.38 }
  ]
})

const bottomCorrelations = computed(() => {
  return [
    { rank: 1, symbol: 'USDT', correlation: -0.02, beta: 0.01 },
    { rank: 2, symbol: 'USDC', correlation: -0.01, beta: 0.02 },
    { rank: 3, symbol: 'DAI', correlation: 0.05, beta: 0.03 },
    { rank: 4, symbol: 'PAXG', correlation: 0.12, beta: 0.15 },
    { rank: 5, symbol: 'XRP', correlation: 0.18, beta: 0.65 }
  ]
})

async function fetchData() {
  loading.value = true
  error.value = null
  
  try {
    // Mock data generation
    await new Promise(resolve => setTimeout(resolve, 500))
    
    // Generate correlation matrix
    const matrixSize = coins.length
    const corrData = Array(matrixSize).fill(null).map(() => 
      Array(matrixSize).fill(null).map(() => Math.random() * 2 - 1)
    )
    // Make diagonal 1
    for (let i = 0; i < matrixSize; i++) {
      corrData[i][i] = 1
    }
    
    correlationMatrix.value = {
      labels: coins,
      data: corrData
    }
    
    // Generate beta matrix
    const betaData = Array(matrixSize).fill(null).map(() => 
      Math.random() * 2 // Beta between 0 and 2
    )
    
    betaMatrix.value = {
      labels: coins,
      data: betaData.map(b => [b]) // Single column for beta vs BTC
    }
    
    // Generate distributions
    corrDistribution.value = {
      buckets: [-1, -0.8, -0.6, -0.4, -0.2, 0, 0.2, 0.4, 0.6, 0.8, 1],
      counts: [2, 3, 5, 8, 12, 18, 22, 15, 10, 4, 1]
    }
    
    betaDistribution.value = {
      buckets: [0, 0.25, 0.5, 0.75, 1, 1.25, 1.5, 1.75, 2],
      counts: [5, 10, 15, 25, 30, 20, 15, 8, 2]
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
/* Matrix containers */
.matrix-container {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  height: 100%;
}

.matrix-legend {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  font-size: 12px;
  color: var(--text-secondary);
  padding: var(--space-sm) 0;
}

.legend-gradient {
  flex: 1;
  height: 8px;
  border-radius: 4px;
}

.legend-gradient.correlation {
  background: linear-gradient(to right, #ef4444, #f59e0b, #eab308, #84cc16, #22c55e);
}

.legend-gradient.beta {
  background: linear-gradient(to right, #3b82f6, #8b5cf6, #ec4899);
}

/* Distribution containers */
.distribution-container {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  height: 100%;
}

.stats-panel {
  display: flex;
  justify-content: space-around;
  padding: var(--space-md);
  background: var(--bg-primary);
  border-radius: 6px;
}

.stat-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.stat-item .label {
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: uppercase;
}

.stat-item .value {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

/* Correlations table */
.correlations-table {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(350px, 1fr));
  gap: var(--space-xl);
  padding: var(--space-sm);
}

.table-section h4 {
  margin: 0 0 var(--space-md) 0;
  color: var(--text-secondary);
  font-size: 13px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.correlation-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.corr-row {
  display: grid;
  grid-template-columns: 30px 80px 80px 80px 1fr;
  align-items: center;
  padding: 8px 12px;
  background: var(--bg-primary);
  border-radius: 6px;
  font-size: 13px;
  transition: all 0.2s ease;
}

.corr-row:hover {
  background: var(--bg-tertiary);
  transform: translateX(2px);
}

.corr-row .rank {
  color: var(--text-secondary);
  font-weight: 500;
}

.corr-row .symbol {
  font-weight: 700;
  color: var(--text-primary);
}

.corr-row .correlation {
  font-weight: 600;
  font-family: var(--font-mono);
}

.corr-row .correlation.positive {
  color: var(--color-positive);
}

.corr-row .correlation.negative {
  color: var(--color-negative);
}

.corr-row .beta {
  color: var(--text-secondary);
  font-size: 12px;
  font-family: var(--font-mono);
}

.bar-container {
  position: relative;
  height: 16px;
  background: var(--bg-secondary);
  border-radius: 3px;
  overflow: hidden;
}

.bar {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  background: var(--color-positive);
  opacity: 0.8;
  transition: width 0.3s ease;
}

.bar.negative {
  right: 0;
  left: auto;
  background: var(--color-negative);
}

/* Responsive adjustments */
@media (max-width: 1200px) {
  .correlations-table {
    grid-template-columns: 1fr;
  }
}
</style>