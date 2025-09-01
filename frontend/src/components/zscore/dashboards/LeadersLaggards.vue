<template>
  <div class="leaders-laggards">
    <div class="dashboard-header">
      <h2>Cross-Sectional Leaders & Laggards</h2>
      <div class="header-controls">
        <select v-model="topN" @change="fetchData">
          <option :value="50">Top 50</option>
          <option :value="100">Top 100</option>
          <option :value="200">Top 200</option>
        </select>
        <select v-model="timeframe" @change="fetchData">
          <option value="1h">1 Hour</option>
          <option value="24h">24 Hours</option>
          <option value="7d">7 Days</option>
        </select>
        <button @click="fetchData" class="update-btn">Update</button>
      </div>
    </div>

    <div class="dashboard-grid">
      <!-- Lead-Lag Matrix -->
      <MetricCard 
        title="Lead-Lag Matrix"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="lead-lag-container">
          <div class="matrix-controls">
            <select v-model="leadLagPair.coin1">
              <option v-for="coin in topCoins" :key="coin">{{ coin }}</option>
            </select>
            <span class="vs">vs</span>
            <select v-model="leadLagPair.coin2">
              <option v-for="coin in topCoins" :key="coin">{{ coin }}</option>
            </select>
          </div>
          <div class="lead-lag-chart">
            <HeatmapChart
              v-if="leadLagMatrix"
              :data="leadLagMatrix"
              :min="-1"
              :max="1"
              color-scheme="correlation"
            />
          </div>
          <div class="lag-labels">
            <span>← {{ leadLagPair.coin1 }} leads</span>
            <span>0h</span>
            <span>{{ leadLagPair.coin2 }} leads →</span>
          </div>
        </div>
      </MetricCard>

      <!-- Momentum Leaders/Laggards -->
      <MetricCard 
        title="Momentum Rankings"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="momentum-rankings">
          <div class="ranking-tabs">
            <button 
              :class="['tab', { active: activeTab === 'leaders' }]"
              @click="activeTab = 'leaders'"
            >
              Leaders
            </button>
            <button 
              :class="['tab', { active: activeTab === 'laggards' }]"
              @click="activeTab = 'laggards'"
            >
              Laggards
            </button>
          </div>
          
          <div class="ranking-list">
            <div 
              v-for="coin in (activeTab === 'leaders' ? leaders : laggards)" 
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
                  <span class="label">{{ timeframe }} Ret</span>
                  <span class="value" :class="coin.returns > 0 ? 'positive' : 'negative'">
                    {{ coin.returns > 0 ? '+' : '' }}{{ coin.returns.toFixed(1) }}%
                  </span>
                </div>
                <div class="metric">
                  <span class="label">Vol Z</span>
                  <span class="value">{{ coin.volumeZScore.toFixed(2) }}</span>
                </div>
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
        @retry="fetchData"
      >
        <div class="volume-spikes">
          <div class="spike-threshold">
            <span>Threshold: </span>
            <input 
              type="number" 
              v-model.number="spikeThreshold" 
              @change="updateVolumeSpikes"
              min="2"
              max="5"
              step="0.5"
            > σ
          </div>
          <div class="spike-chart">
            <canvas ref="volumeSpikeChart"></canvas>
          </div>
          <div class="spike-count">
            {{ volumeSpikes.length }} assets with volume > {{ spikeThreshold }}σ
          </div>
        </div>
      </MetricCard>

      <!-- Decorrelation Alert -->
      <MetricCard 
        title="Decorrelation Alert"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="decorrelation-list">
          <div class="correlation-threshold">
            <span>Alert below: </span>
            <input 
              type="number" 
              v-model.number="correlationThreshold" 
              @change="updateDecorrelation"
              min="0"
              max="0.5"
              step="0.05"
            >
          </div>
          <div class="alert-list">
            <div 
              v-for="item in decorrelatedAssets" 
              :key="item.symbol"
              class="alert-item"
            >
              <span class="symbol">{{ item.symbol }}</span>
              <span class="correlation">
                r = {{ item.correlation.toFixed(3) }}
              </span>
              <span class="status" :class="item.status">
                {{ item.status }}
              </span>
            </div>
          </div>
        </div>
      </MetricCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Chart } from 'chart.js'
import MetricCard from '../components/common/MetricCard.vue'
import HeatmapChart from '../components/charts/HeatmapChart.vue'
import { useZScoreData } from '@/composables/useZScoreData'
import { formatNumber } from '@/utils/formatters'

// Define proper interfaces for the expected data structure
interface CoinRanking {
  symbol: string
  zscore: number
  returns: { [key: string]: number }
  volumeZScore: number
  volumeChange: number
}

interface ZScoreData {
  rankings?: CoinRanking[]
  leadLagData?: { [key: string]: any }
  correlations?: { [key: string]: number }
}

interface CoinMetrics {
  symbol: string
  rank: number
  zscore: number
  returns: number
  volumeZScore: number
}

interface VolumeSpike {
  symbol: string
  zscore: number
  percentChange: number
}

interface DecorrelatedAsset {
  symbol: string
  correlation: number
  status: 'warning' | 'alert' | 'critical'
}

interface FetchDataParams {
  topN?: number
  timeframe?: string
  period?: string
  exchange?: string
  includeLeadLag?: boolean
}

const topN = ref(100)
const timeframe = ref('24h')
const activeTab = ref<'leaders' | 'laggards'>('leaders')
const spikeThreshold = ref(3)
const correlationThreshold = ref(0.3)

const leadLagPair = ref({
  coin1: 'BTC',
  coin2: 'ETH'
})

const {
  data,
  loading,
  error,
  fetchData: fetchZScoreData
} = useZScoreData()

// Type assertion to ensure data has the correct structure
const typedData = computed((): ZScoreData => {
  return data.value as ZScoreData || {}
})

const topCoins = computed(() => {
  if (!typedData.value.rankings) return []
  return typedData.value.rankings.slice(0, 10).map((r: CoinRanking) => r.symbol)
})

const leaders = computed((): CoinMetrics[] => {
  if (!typedData.value.rankings) return []
  return typedData.value.rankings
    .slice(0, 10)
    .map((r: CoinRanking, i: number) => ({
      symbol: r.symbol,
      rank: i + 1,
      zscore: r.zscore,
      returns: r.returns[timeframe.value] || 0,
      volumeZScore: r.volumeZScore
    }))
})

const laggards = computed((): CoinMetrics[] => {
  if (!typedData.value.rankings) return []
  const sorted = [...typedData.value.rankings].reverse()
  return sorted
    .slice(0, 10)
    .map((r: CoinRanking, i: number) => ({
      symbol: r.symbol,
      rank: (typedData.value.rankings?.length || 0) - i,
      zscore: r.zscore,
      returns: r.returns[timeframe.value] || 0,
      volumeZScore: r.volumeZScore
    }))
})

const leadLagMatrix = computed(() => {
  if (!typedData.value.leadLagData) return null
  const { coin1, coin2 } = leadLagPair.value
  return typedData.value.leadLagData[`${coin1}_${coin2}`]
})

const volumeSpikes = computed((): VolumeSpike[] => {
  if (!typedData.value.rankings) return []
  return typedData.value.rankings
    .filter((r: CoinRanking) => Math.abs(r.volumeZScore) > spikeThreshold.value)
    .map((r: CoinRanking) => ({
      symbol: r.symbol,
      zscore: r.volumeZScore,
      percentChange: r.volumeChange
    }))
    .sort((a: VolumeSpike, b: VolumeSpike) => Math.abs(b.zscore) - Math.abs(a.zscore))
})

const decorrelatedAssets = computed((): DecorrelatedAsset[] => {
  if (!typedData.value.correlations) return []
  return Object.entries(typedData.value.correlations)
    .filter(([, corr]) => Math.abs(corr as number) < correlationThreshold.value)
    .map(([symbol, corr]) => ({
      symbol,
      correlation: corr as number,
      status: getCorrelationStatus(corr as number)
    }))
    .sort((a: DecorrelatedAsset, b: DecorrelatedAsset) => Math.abs(a.correlation) - Math.abs(b.correlation))
})

const volumeSpikeChart = ref<HTMLCanvasElement>()
let chartInstance: Chart | null = null

const getCorrelationStatus = (corr: number): 'warning' | 'alert' | 'critical' => {
  const absCorr = Math.abs(corr)
  if (absCorr < 0.1) return 'critical'
  if (absCorr < 0.2) return 'alert'
  return 'warning'
}

const updateVolumeSpikes = () => {
  if (!chartInstance || !volumeSpikes.value.length) return
  
  chartInstance.data.labels = volumeSpikes.value.map((v: VolumeSpike) => v.symbol)
  chartInstance.data.datasets[0].data = volumeSpikes.value.map((v: VolumeSpike) => v.zscore)
  chartInstance.update()
}

const updateDecorrelation = () => {
  // Trigger reactivity
}

const createVolumeSpikeChart = () => {
  if (!volumeSpikeChart.value) return
  
  const ctx = volumeSpikeChart.value.getContext('2d')
  if (!ctx) return
  
  chartInstance = new Chart(ctx, {
    type: 'bar',
    data: {
      labels: volumeSpikes.value.map((v: VolumeSpike) => v.symbol),
      datasets: [{
        label: 'Volume Z-Score',
        data: volumeSpikes.value.map((v: VolumeSpike) => v.zscore),
        backgroundColor: volumeSpikes.value.map((v: VolumeSpike) => 
          v.zscore > 0 ? 'rgba(16, 185, 129, 0.8)' : 'rgba(239, 68, 68, 0.8)'
        ),
        borderColor: volumeSpikes.value.map((v: VolumeSpike) => 
          v.zscore > 0 ? 'rgb(16, 185, 129)' : 'rgb(239, 68, 68)'
        ),
        borderWidth: 1
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: { display: false },
        tooltip: {
          callbacks: {
            afterLabel: (context) => {
              const spike = volumeSpikes.value[context.dataIndex]
              return `${spike.percentChange > 0 ? '+' : ''}${spike.percentChange.toFixed(1)}% change`
            }
          }
        }
      },
      scales: {
        y: {
          beginAtZero: true,
          title: { display: true, text: 'Z-Score' }
        }
      }
    }
  })
}

const fetchData = async () => {
  const params: FetchDataParams = {
    topN: topN.value,
    timeframe: timeframe.value,
    period: timeframe.value,
    exchange: 'binance',
    includeLeadLag: true
  }
  
  await fetchZScoreData(params)
}

watch(volumeSpikes, () => {
  if (chartInstance) {
    updateVolumeSpikes()
  }
})

onMounted(() => {
  fetchData()
  setTimeout(createVolumeSpikeChart, 100)
})

onUnmounted(() => {
  if (chartInstance) {
    chartInstance.destroy()
  }
})
</script>


