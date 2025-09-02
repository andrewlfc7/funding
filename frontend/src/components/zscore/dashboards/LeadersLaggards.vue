<template>
  <div class="leaders-laggards">
    <div class="dashboard-header">
      <h2>Cross-Sectional Leaders & Laggards</h2>
      <div class="header-controls">
        <select v-model="topN" @change="updateData">
          <option :value="10">Top 10</option>
          <option :value="20">Top 20</option>
          <option :value="30">Top 30</option>
          <option :value="50">Top 50</option>
        </select>
        <select v-model="timeframe" @change="updateData">
          <option value="1h">1 Hour</option>
          <option value="4h">4 Hours</option>
          <option value="1d">1 Day</option>
        </select>
        <select v-model="period" @change="updateData">
          <option value="7d">7 Days</option>
          <option value="30d">30 Days</option>
                    <option value="90d">90 Days</option>
        </select>
        <button @click="updateData" class="update-btn" :disabled="loading">
          {{ loading ? 'Loading...' : 'Update' }}
        </button>
      </div>
    </div>

    <div v-if="error" class="error-message">
      {{ error }}
    </div>

    <div class="dashboard-grid">
      <!-- Market Overview Stats -->
      <MetricCard 
        title="Market Overview"
        class="full-width"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="market-stats">
          <div class="stat-item">
            <span class="label">Mean Z-Score</span>
            <span class="value">{{ marketStats.meanZScore.toFixed(3) }}</span>
          </div>
          <div class="stat-item">
            <span class="label">Std Dev</span>
            <span class="value">{{ marketStats.stdZScore.toFixed(3) }}</span>
          </div>
          <div class="stat-item">
            <span class="label">Bullish (>1σ)</span>
            <span class="value positive">{{ marketStats.bullishCount }}</span>
          </div>
          <div class="stat-item">
            <span class="label">Bearish (<-1σ)</span>
            <span class="value negative">{{ marketStats.bearishCount }}</span>
          </div>
          <div class="stat-item">
            <span class="label">Market Breadth</span>
            <span class="value" :class="marketStats.marketBreadth > 0 ? 'positive' : 'negative'">
              {{ marketStats.marketBreadth.toFixed(1) }}%
            </span>
          </div>
        </div>
      </MetricCard>

      <!-- Momentum Leaders -->
      <MetricCard 
        title="Momentum Leaders"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="ranking-list">
          <div 
            v-for="coin in leaders.slice(0, 10)" 
            :key="coin.symbol"
            class="ranking-item"
          >
            <span class="rank">#{{ coin.rank }}</span>
            <span class="symbol">{{ coin.symbol }}</span>
            <div class="metrics">
              <div class="metric">
                <span class="label">Z-Score</span>
                <span class="value" :class="coin.zscore > 0 ? 'positive' : 'negative'">
                  {{ coin.zscore > 0 ? '+' : '' }}{{ coin.zscore.toFixed(2) }}
                </span>
              </div>
              <div class="metric">
                <span class="label">Returns</span>
                <span class="value" :class="coin.returns > 0 ? 'positive' : 'negative'">
                  {{ formatReturns(coin.returns) }}
                </span>
              </div>
              <div class="metric">
                <span class="label">Volume</span>
                <span class="value">{{ formatVolume(coin.volume) }}</span>
              </div>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Momentum Laggards -->
      <MetricCard 
        title="Momentum Laggards"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="ranking-list">
          <div 
            v-for="coin in laggards.slice(0, 10)" 
            :key="coin.symbol"
            class="ranking-item"
          >
            <span class="rank">#{{ coin.rank }}</span>
            <span class="symbol">{{ coin.symbol }}</span>
            <div class="metrics">
              <div class="metric">
                <span class="label">Z-Score</span>
                <span class="value" :class="coin.zscore > 0 ? 'positive' : 'negative'">
                  {{ coin.zscore > 0 ? '+' : '' }}{{ coin.zscore.toFixed(2) }}
                </span>
              </div>
              <div class="metric">
                <span class="label">Returns</span>
                <span class="value" :class="coin.returns > 0 ? 'positive' : 'negative'">
                  {{ formatReturns(coin.returns) }}
                </span>
              </div>
              <div class="metric">
                <span class="label">Volume</span>
                <span class="value">{{ formatVolume(coin.volume) }}</span>
              </div>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Volume Spike Detection -->
      <MetricCard 
        title="Volume Spike Detection"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="volume-spikes">
          <div class="spike-threshold">
            <span>Showing assets with |volume Z-score| > </span>
            <input 
              type="number" 
              v-model.number="spikeThreshold" 
              min="1.5"
              max="3"
              step="0.5"
            > σ
          </div>
          <div class="spike-list">
            <div 
              v-for="spike in filteredVolumeSpikes" 
              :key="spike.symbol"
              class="spike-item"
            >
              <span class="symbol">{{ spike.symbol }}</span>
              <span class="zscore" :class="spike.volumeZScore > 0 ? 'positive' : 'negative'">
                {{ spike.volumeZScore.toFixed(2) }}σ
              </span>
              <span class="change" :class="spike.priceChange > 0 ? 'positive' : 'negative'">
                {{ formatReturns(spike.priceChange) }}
              </span>
            </div>
            <div v-if="filteredVolumeSpikes.length === 0" class="no-data">
              No volume spikes above threshold
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Decorrelation Alert -->
      <MetricCard 
        title="Decorrelation Analysis"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="decorrelation-container">
          <div class="correlation-threshold">
            <span>Market correlation threshold: </span>
            <input 
              type="number" 
              v-model.number="correlationThreshold" 
              min="0"
              max="0.8"
              step="0.1"
            >
          </div>
          <div class="alert-list">
            <div 
              v-for="asset in filteredDecorrelatedAssets" 
              :key="asset.symbol"
              class="alert-item"
              :class="getDecorrelationStatus(asset.correlationWithMarket)"
            >
              <span class="symbol">{{ asset.symbol }}</span>
              <div class="correlation-values">
                <div class="corr-item">
                  <span class="label">Market r:</span>
                  <span class="value">{{ asset.correlationWithMarket.toFixed(3) }}</span>
                </div>
                <div class="corr-item">
                  <span class="label">Avg r:</span>
                  <span class="value">{{ asset.avgCorrelation.toFixed(3) }}</span>
                </div>
              </div>
              <span class="status-badge" :class="getDecorrelationStatus(asset.correlationWithMarket)">
                {{ getDecorrelationStatus(asset.correlationWithMarket) }}
              </span>
            </div>
            <div v-if="filteredDecorrelatedAssets.length === 0" class="no-data">
              No assets below correlation threshold
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Lead-Lag Analysis -->
      <MetricCard 
        title="Lead-Lag Correlation Analysis"
        class="full-width"
        :loading="loading"
        :error="error"
        @retry="updateData"
      >
        <div class="lead-lag-container" v-if="leadLagMatrix">
          <div class="matrix-controls">
            <select v-model="leadLagPair.coin1">
              <option v-for="coin in leadLagMatrix.coins" :key="coin" :value="coin">
                {{ coin }}
              </option>
            </select>
            <span class="vs">vs</span>
            <select v-model="leadLagPair.coin2">
              <option v-for="coin in leadLagMatrix.coins" :key="coin" :value="coin">
                {{ coin }}
              </option>
            </select>
          </div>
          
          <div class="lead-lag-chart" v-if="leadLagData">
            <canvas ref="leadLagChart"></canvas>
            <div class="lead-lag-info">
              <div class="info-item">
                <span class="label">Optimal Lag:</span>
                <span class="value">{{ leadLagData.optimalLag }}h</span>
              </div>
              <div class="info-item">
                <span class="label">Max Correlation:</span>
                <span class="value">{{ leadLagData.maxCorrelation.toFixed(3) }}</span>
              </div>
              <div class="info-item" v-if="leadLagData.optimalLag !== 0">
                <span class="label">Relationship:</span>
                <span class="value">
                  {{ leadLagData.optimalLag < 0 ? leadLagPair.coin1 : leadLagPair.coin2 }} leads by {{ Math.abs(leadLagData.optimalLag) }}h
                </span>
              </div>
            </div>
          </div>
          <div v-else class="no-data">
            Select two different coins to analyze lead-lag relationship
          </div>
        </div>
      </MetricCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from 'vue'
import { Chart } from 'chart.js'
import { useLeadersLaggards } from '@/composables/useLeadersLaggards'
import MetricCard from '../components/common/MetricCard.vue'

// State
const topN = ref(30)
const timeframe = ref('1h')
const period = ref('90d')
const spikeThreshold = ref(2)
const correlationThreshold = ref(0.5)

const leadLagPair = ref({
  coin1: 'BTC',
  coin2: 'ETH'
})

// Chart refs
const leadLagChart = ref<HTMLCanvasElement>()
let leadLagChartInstance: Chart | null = null

// Composable
const {
  loading,
  error,
  leaders,
  laggards,
  volumeSpikes,
  decorrelatedAssets,
  leadLagMatrix,
  marketStats,
  fetchData,
  formatReturns,
  formatVolume,
  getLeadLagData,
  getDecorrelationStatus
} = useLeadersLaggards()

// Computed
const filteredVolumeSpikes = computed(() => {
  return volumeSpikes.value.filter(spike => 
    Math.abs(spike.volumeZScore) > spikeThreshold.value
  )
})

const filteredDecorrelatedAssets = computed(() => {
  return decorrelatedAssets.value.filter(asset => 
    asset.correlationWithMarket < correlationThreshold.value
  )
})

const leadLagData = computed(() => {
  if (!leadLagMatrix.value || leadLagPair.value.coin1 === leadLagPair.value.coin2) {
    return null
  }
  return getLeadLagData(leadLagPair.value.coin1, leadLagPair.value.coin2)
})

// Methods
async function updateData() {
  await fetchData({
    exchange: 'binance',
    marketType: 'spot',
    timeframe: timeframe.value,
    period: period.value,
    topN: topN.value
  })
  
  // Update lead-lag chart after data is loaded
  await nextTick()
  updateLeadLagChart()
}

function createLeadLagChart() {
  if (!leadLagChart.value || !leadLagData.value) return
  
  const ctx = leadLagChart.value.getContext('2d')
  if (!ctx) return
  
  // Destroy existing chart
  if (leadLagChartInstance) {
    leadLagChartInstance.destroy()
  }
  
  leadLagChartInstance = new Chart(ctx, {
    type: 'line',
    data: {
      labels: leadLagData.value.lags.map(lag => `${lag}h`),
      datasets: [{
        label: `${leadLagPair.value.coin1} vs ${leadLagPair.value.coin2}`,
        data: leadLagData.value.correlations,
        borderColor: 'rgba(59, 130, 246, 1)',
        backgroundColor: 'rgba(59, 130, 246, 0.1)',
        borderWidth: 2,
        tension: 0.4,
        pointRadius: 4,
        pointHoverRadius: 6
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          display: false
        },
        tooltip: {
          callbacks: {
            label: (context) => {
              return `Correlation: ${context.parsed.y.toFixed(3)}`
            }
          }
        }
      },
      scales: {
        x: {
          title: {
            display: true,
            text: 'Lag (hours)',
            color: 'rgba(255, 255, 255, 0.8)'
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.1)'
          },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)'
          }
        },
        y: {
          title: {
            display: true,
            text: 'Correlation',
            color: 'rgba(255, 255, 255, 0.8)'
          },
          min: -1,
          max: 1,
          grid: {
            color: 'rgba(255, 255, 255, 0.1)'
          },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)'
          }
        }
      }
    }
  })
}

function updateLeadLagChart() {
  if (!leadLagData.value) return
  
  if (leadLagChartInstance) {
    leadLagChartInstance.data.labels = leadLagData.value.lags.map(lag => `${lag}h`)
    leadLagChartInstance.data.datasets[0].data = leadLagData.value.correlations
    leadLagChartInstance.data.datasets[0].label = `${leadLagPair.value.coin1} vs ${leadLagPair.value.coin2}`
    leadLagChartInstance.update()
  } else {
    createLeadLagChart()
  }
}

// Lifecycle
onMounted(() => {
  updateData()
})

// Watchers
watch([topN, timeframe, period], () => {
  updateData()
})

watch(leadLagPair, () => {
  updateLeadLagChart()
}, { deep: true })

watch(leadLagData, () => {
  updateLeadLagChart()
})
</script>