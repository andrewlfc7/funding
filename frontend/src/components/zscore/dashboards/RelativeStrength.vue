<template>
  <div class="relative-strength">
    <div class="dashboard-header">
      <h2>Relative Strength Analysis</h2>
      <div class="header-controls">
        <select v-model="baseCoin" @change="fetchData">
          <option value="BTC">BTC</option>
          <option value="ETH">ETH</option>
          <option value="USDT">USDT</option>
        </select>
        <select v-model="period" @change="fetchData">
          <option value="24h">24H</option>
          <option value="7d">7D</option>
          <option value="30d">30D</option>
        </select>
        <button @click="fetchData" class="update-btn" :disabled="loading">
          {{ loading ? 'Loading...' : 'Update' }}
        </button>
      </div>
    </div>

    <div class="dashboard-grid">
      <!-- RS Z-Score Rankings -->
      <MetricCard 
        title="RS Z-Score Rankings"
        :subtitle="`vs ${baseCoin}`"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="rs-rankings">
          <div class="rankings-list">
            <!-- Top performers -->
            <div 
              v-for="coin in topPerformers" 
              :key="coin.symbol"
              class="ranking-item top"
            >
              <span class="rank">{{ coin.rank }}.</span>
              <span class="pair">{{ coin.pair }}</span>
              <span class="zscore" :class="getZScoreClass(coin.rsZScore)">
                {{ formatZScore(coin.rsZScore) }}
              </span>
              <div class="bar-container">
                <div 
                  class="bar"
                  :class="coin.rsZScore > 0 ? 'positive' : 'negative'"
                  :style="{ width: getBarWidth(coin.rsZScore) }"
                ></div>
              </div>
            </div>
            
            <!-- Separator -->
            <div class="ranking-separator" v-if="topPerformers.length > 0 && bottomPerformers.length > 0">
              <span>...</span>
            </div>
            
            <!-- Bottom performers -->
            <div 
              v-for="coin in bottomPerformers" 
              :key="coin.symbol"
              class="ranking-item bottom"
            >
              <span class="rank">{{ coin.rank }}.</span>
              <span class="pair">{{ coin.pair }}</span>
              <span class="zscore" :class="getZScoreClass(coin.rsZScore)">
                {{ formatZScore(coin.rsZScore) }}
              </span>
              <div class="bar-container">
                <div 
                  class="bar negative"
                  :style="{ width: getBarWidth(coin.rsZScore) }"
                ></div>
              </div>
            </div>
          </div>
        </div>
      </MetricCard>

      <!-- Pair Z-Score Divergence -->
      <MetricCard 
        title="Pair Z-Score Divergence"
        subtitle="Spread trading opportunities"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="pair-divergence">
          <div v-if="pairDivergences.length === 0" class="empty-state">
            <p>No significant pair divergences detected</p>
          </div>
          <div v-else-if="selectedDivergence" class="divergence-detail">
            <div class="pair-header">
              <h4>{{ selectedDivergence.pair }} spread</h4>
              <span class="z-value" :class="divergenceClass">
                Z: {{ selectedDivergence.zScore.toFixed(1) }} 
                ({{ divergenceStatus }})
              </span>
            </div>
            
            <TimeSeriesChart
              v-if="divergenceTimeSeries.length > 0"
              :data="divergenceTimeSeries"
              y-field="spread"
              :show-zero-line="true"
              :height="200"
            />
            
            <div class="divergence-info">
              <span>Historical range: ±{{ historicalRange }}σ</span>
              <span v-if="meanReversionExpected" class="reversion-note">
                Mean reversion expected
              </span>
            </div>
          </div>
          
          <div class="pair-selector" v-if="pairDivergences.length > 0">
            <select v-model="selectedPairIndex">
              <option 
                v-for="(div, index) in pairDivergences" 
                :key="div.pair"
                :value="index"
              >
                {{ div.pair }} (Z: {{ div.zScore.toFixed(1) }})
              </option>
            </select>
          </div>
        </div>
      </MetricCard>

      <!-- Momentum Persistence -->
      <MetricCard 
        title="Momentum Persistence"
        subtitle="Z-score autocorrelation"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="momentum-persistence">
          <div 
            v-for="category in momentumPersistence" 
            :key="category.category"
            class="persistence-category"
          >
            <div class="category-header">
              <span class="label">{{ capitalize(category.category) }}</span>
              <span class="description">{{ category.description }}</span>
            </div>
            <div class="bar-wrapper">
              <div 
                class="persistence-bar"
                :class="category.category"
                :style="{ width: category.correlation * 100 + '%' }"
              ></div>
            </div>
            <div class="coin-list">
              {{ formatCoinList(category.coins) }}
            </div>
          </div>
          <div v-if="momentumPersistence.length === 0" class="empty-state">
            <p>Calculating momentum persistence...</p>
          </div>
        </div>
      </MetricCard>

      <!-- Cross-Sectional Momentum -->
      <MetricCard 
        title="Cross-Sectional Momentum"
        subtitle="Factor Exposure"
        :loading="loading"
        :error="error"
        @retry="fetchData"
      >
        <div class="momentum-factors">
          <div class="factor-header">
            <h4>Mom Factor Loading</h4>
            <span class="subtitle">High β coins:</span>
          </div>
          <div class="factor-list">
            <div 
              v-for="factor in momentumFactors" 
              :key="factor.symbol"
              class="factor-item"
            >
              <span class="symbol">{{ factor.symbol }}</span>
              <span class="beta">
                β = {{ factor.beta.toFixed(2) }}
                <span class="r2">(R² = {{ (factor.r2 * 100).toFixed(0) }}%)</span>
              </span>
              <div class="loading-bar">
                <div 
                  class="bar"
                  :class="factor.loading > 0 ? 'positive' : 'negative'"
                  :style="{ 
                    width: Math.min(Math.abs(factor.loading) * 30, 100) + '%'
                  }"
                ></div>
              </div>
            </div>
          </div>
          <div v-if="momentumFactors.length === 0" class="empty-state">
            <p>No high beta coins found</p>
          </div>
        </div>
      </MetricCard>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRelativeStrength } from '@/composables/useRelativeStrength'
import MetricCard from '../components/common/MetricCard.vue'
import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'

const {
  loading,
  error,
  baseCoin,
  period,
  exchange,
  rsRankings,
  topPerformers,
  bottomPerformers,
  pairDivergences,
  momentumPersistence,
  momentumFactors,
  fetchData
} = useRelativeStrength()

// Local state
const selectedPairIndex = ref(0)

// Computed properties
const selectedDivergence = computed(() => 
  pairDivergences.value[selectedPairIndex.value]
)

const divergenceTimeSeries = computed(() => {
  if (!selectedDivergence.value) return []
  return selectedDivergence.value.timeSeries.map(t => ({
    timestamp: t.timestamp * 1000,
    spread: t.spread
  }))
})

const divergenceClass = computed(() => {
  const z = selectedDivergence.value?.zScore || 0
  if (z > 2.5) return 'extreme'
  if (z > 2) return 'high'
  if (z > 1.5) return 'moderate'
  return 'normal'
})

const divergenceStatus = computed(() => {
  const z = selectedDivergence.value?.zScore || 0
  if (z > 2.5) return 'extreme wide'
  if (z > 2) return 'wide'
  if (z > 1.5) return 'diverging'
  return 'normal'
})

const historicalRange = computed(() => {
  if (!selectedDivergence.value) return '0.0'
  const range = selectedDivergence.value.historicalRange
  return Math.max(Math.abs(range.min), Math.abs(range.max)).toFixed(1)
})

const meanReversionExpected = computed(() => {
  const z = selectedDivergence.value?.zScore || 0
  return z > 2
})

// Helper functions
const capitalize = (str: string) => 
  str.charAt(0).toUpperCase() + str.slice(1)

const getZScoreClass = (zscore: number | null) => {
  if (zscore === null) return 'neutral'
  if (zscore > 0) return 'positive'
  if (zscore < 0) return 'negative'
  return 'neutral'
}

const formatZScore = (zscore: number | null) => {
  if (zscore === null) return 'N/A'
  const formatted = zscore.toFixed(1)
  return zscore > 0 ? `+${formatted}` : formatted
}

const getBarWidth = (zscore: number | null) => {
  if (zscore === null) return '0%'
  return (Math.min(Math.abs(zscore) / 3, 1) * 100) + '%'
}

const formatCoinList = (coins: string[]) => {
  if (coins.length === 0) return 'None'
  if (coins.length <= 5) return coins.join(', ')
  return `${coins.slice(0, 5).join(', ')} (+${coins.length - 5} more)`
}

// Reset selected pair when pairs change
watch(pairDivergences, (newPairs) => {
  if (selectedPairIndex.value >= newPairs.length) {
    selectedPairIndex.value = 0
  }
})

onMounted(() => {
  fetchData()
})
</script>
