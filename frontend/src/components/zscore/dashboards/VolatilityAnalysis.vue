<template>
  <div class="volatility-analysis">
    <div class="dashboard-header">
      <h2>Volatility Analysis</h2>
      <div class="header-controls">
        <select v-model="topN" @change="fetchData">
          <option :value="30">Top 30</option>
          <option :value="50">Top 50</option>
          <option :value="100">Top 100</option>
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
          <option value="120d">120 Days</option>
        </select>
        <select v-model="exchange" @change="fetchData" :disabled="metaLoading">
          <option v-for="ex in exchanges" :key="ex" :value="ex">
            {{ ex }}
          </option>
        </select>
        <button @click="fetchData" class="update-btn" :disabled="loading || metaLoading">
          {{ loading ? 'Loading...' : 'Update' }}
        </button>
      </div>
    </div>

    <div v-if="!metaLoading" class="dashboard-grid">
      <!-- 1. Volatility Z-Score Time Series -->
      <MetricCard 
        title="Volatility Z-Score Time Series"
        :span="2"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <TimeSeriesChart
          v-if="hasData"
          :data="chartData"
          :y-field="'volatilityZScore'"
          :y-label="'Volatility Z-Score'"
          :group-by="'symbol'"
        />
      </MetricCard>

      <!-- 2. Vol Z-Score vs Returns -->
      <MetricCard 
        title="Vol Z-Score vs Returns"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <ScatterChart
          v-if="volZScoreVsReturns.length > 0"
          :data="volZScoreVsReturns"
          x-field="returns"
          y-field="volatilityZScore"
          label-field="symbol"
          x-label="Returns (%)"
          y-label="Volatility Z-Score"
          :show-labels="true"
        />
      </MetricCard>

      <!-- 3. Daily Range Distribution -->
      <MetricCard 
        title="Daily Range Distribution"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <HistogramChart
          v-if="rangeDistribution"
          :data="rangeDistribution"
        />
      </MetricCard>

      <!-- 4. Volume Z-Score vs Daily Range -->
      <MetricCard 
        title="Volume Z-Score vs Daily Range"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <ScatterChart
          v-if="volumeZScoreVsRange.length > 0"
          :data="volumeZScoreVsRange"
          x-field="volumeZScore"
          y-field="range"
          label-field="symbol"
          x-label="Volume Z-Score"
          y-label="Daily Range (%)"
          :color-field="'volatilityZScore'"
          :show-labels="true"
        />
      </MetricCard>

      <!-- 5. Realized Vol Percentiles (full width) -->
      <MetricCard 
        title="Realized Vol Percentiles"
        :span="2"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="vol-percentiles">
          <div class="percentile-chart">
            <PercentileChart
              v-if="realizedVolPercentiles.length > 0"
              :data="realizedVolPercentiles"
            />
          </div>
          <div class="percentile-stats">
            <h4>Current Statistics</h4>
            <div class="stats-list">
              <div class="stat-item">
                <span class="label">Mean Volatility</span>
                <span class="value">{{ stats.meanVolatility.toFixed(1) }}%</span>
              </div>
              <div class="stat-item">
                <span class="label">Max Volatility</span>
                <span class="value high">{{ stats.maxVolatility.toFixed(1) }}%</span>
              </div>
              <div class="stat-item">
                <span class="label">Min Volatility</span>
                <span class="value low">{{ stats.minVolatility.toFixed(1) }}%</span>
              </div>
              <div class="stat-item">
                <span class="label">High Vol Coins</span>
                <span class="value high">{{ stats.highVolCount }}</span>
              </div>
              <div class="stat-item">
                <span class="label">Low Vol Coins</span>
                <span class="value low">{{ stats.lowVolCount }}</span>
              </div>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- High/Low Volatility Rankings -->
      <MetricCard 
        title="Volatility Rankings"
        :span="2"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="vol-rankings">
          <div class="ranking-section">
            <h4>High Volatility (Z > 1.5)</h4>
            <div class="coin-grid">
              <div v-for="coin in highVolCoins" :key="coin.symbol" class="vol-card">
                <div class="coin-header">
                  <span class="symbol">{{ coin.symbol }}</span>
                  <span class="vol-zscore high">{{ coin.volatilityZScore.toFixed(2) }}σ</span>
                </div>
                <div class="coin-stats">
                  <div class="stat">
                    <span class="label">Vol</span>
                    <span class="value">{{ coin.volatility.toFixed(1) }}%</span>
                  </div>
                  <div class="stat">
                    <span class="label">Range</span>
                    <span class="value">{{ coin.range.toFixed(1) }}%</span>
                  </div>
                  <div class="stat">
                    <span class="label">Returns</span>
                    <span class="value" :class="coin.returns > 0 ? 'positive' : 'negative'">
                      {{ coin.returns > 0 ? '+' : '' }}{{ coin.returns.toFixed(1) }}%
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="ranking-section">
            <h4>Low Volatility (Z < -1.5)</h4>
            <div class="coin-grid">
              <div v-for="coin in lowVolCoins" :key="coin.symbol" class="vol-card">
                <div class="coin-header">
                  <span class="symbol">{{ coin.symbol }}</span>
                  <span class="vol-zscore low">{{ coin.volatilityZScore.toFixed(2) }}σ</span>
                </div>
                <div class="coin-stats">
                  <div class="stat">
                    <span class="label">Vol</span>
                    <span class="value">{{ coin.volatility.toFixed(1) }}%</span>
                  </div>
                  <div class="stat">
                    <span class="label">Range</span>
                    <span class="value">{{ coin.range.toFixed(1) }}%</span>
                  </div>
                  <div class="stat">
                    <span class="label">Returns</span>
                    <span class="value" :class="coin.returns > 0 ? 'positive' : 'negative'">
                      {{ coin.returns > 0 ? '+' : '' }}{{ coin.returns.toFixed(1) }}%
                    </span>
                  </div>
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
import { ref, onMounted } from 'vue'
import { useVolatilityData } from '@/composables/useVolatilityData'
import { useMetaData } from '@/composables/useMetaData'
import MetricCard from '../components/common/MetricCard.vue'
import ScatterChart from '../components/charts/ScatterChart.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'
import HistogramChart from '../components/charts/HistogramChart.vue'
import PercentileChart from '../components/charts/PercentileChart.vue'
import LoadingSpinner from '../components/common/LoadingSpinner.vue'

// Meta data
const { exchanges, defaultExchange, loading: metaLoading, loadMetaData } = useMetaData()

// Controls
const topN = ref(50)
const timeframe = ref('1h')
const period = ref('30d')
const exchange = ref('')

// Use the volatility composable
const {
  loading,
  error,
  hasData,
  chartData,
  volZScoreVsReturns,
  rangeDistribution,
  volumeZScoreVsRange,
  realizedVolPercentiles,
  highVolCoins,
  lowVolCoins,
  stats,
  fetchData: fetch
} = useVolatilityData()

async function fetchData() {
  if (!exchange.value) return
  
  await fetch({
    timeframe: timeframe.value,
    period: period.value,
    exchange: exchange.value,
    topN: topN.value
  })
}

// Initialize on mount
onMounted(async () => {
  await loadMetaData('spot')
  exchange.value = defaultExchange.value
  await fetchData()
})
</script>

<style scoped>
.vol-percentiles {
  display: grid;
  grid-template-columns: 3fr 1fr;
  gap: var(--space-lg);
  height: 100%;
}

.percentile-chart {
  height: 100%;
  min-height: 300px;
}

.percentile-stats {
  background: var(--bg-primary);
  border-radius: 8px;
  padding: var(--space-lg);
}

.percentile-stats h4 {
  margin: 0 0 var(--space-md) 0;
  color: var(--text-secondary);
  font-size: 14px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.stats-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

.stat-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-sm);
  background: var(--bg-secondary);
  border-radius: 6px;
}

.stat-item .label {
  font-size: 13px;
  color: var(--text-secondary);
}

.stat-item .value {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.stat-item .value.high {
  color: #ef4444;
}

.stat-item .value.low {
  color: #10b981;
}

.vol-rankings {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-xl);
}

.ranking-section h4 {
  margin: 0 0 var(--space-md) 0;
  color: var(--text-secondary);
  font-size: 14px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.coin-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: var(--space-md);
}

.vol-card {
  background: var(--bg-primary);
  border: 1px solid var(--border-primary);
  border-radius: 8px;
  padding: var(--space-md);
  transition: all 0.2s ease;
}

.vol-card:hover {
  border-color: var(--border-secondary);
  transform: translateY(-2px);
}

.coin-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: var(--space-sm);
  padding-bottom: var(--space-sm);
  border-bottom: 1px solid var(--border-primary);
}

.coin-header .symbol {
  font-weight: 700;
  font-size: 16px;
  color: var(--text-primary);
}

.vol-zscore {
  font-size: 14px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 4px;
}

.vol-zscore.high {
  background: rgba(239, 68, 68, 0.2);
  color: #ef4444;
}

.vol-zscore.low {
  background: rgba(34, 197, 94, 0.2);
  color: #22c55e;
}

.coin-stats {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: var(--space-sm);
}

.stat {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

.stat .label {
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: uppercase;
  margin-bottom: 2px;
}

.stat .value {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
}

.stat .value.positive {
  color: var(--color-positive);
}

.stat .value.negative {
  color: var(--color-negative);
}

@media (max-width: 1200px) {
  .vol-rankings {
    grid-template-columns: 1fr;
  }
  
  .vol-percentiles {
    grid-template-columns: 1fr;
  }
  
  .percentile-stats {
    margin-top: var(--space-lg);
  }
}

@media (max-width: 768px) {
  .coin-grid {
    grid-template-columns: 1fr;
  }
  
  .vol-card {
    padding: var(--space-sm);
  }
  
  .coin-stats {
    gap: var(--space-xs);
  }
}
</style>