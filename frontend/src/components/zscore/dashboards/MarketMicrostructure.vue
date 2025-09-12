<template>
  <div class="dashboard-header">
    <h2>Market Microstructure</h2>
    <div class="header-controls">
      <select v-model="timeframe" @change="fetchData">
        <option value="5min">5min</option>
        <option value="15min">15min</option>
        <option value="1h">1H</option>
      </select>
      <select v-model="topN" @change="fetchData">
        <option :value="30">Top 30</option>
        <option :value="50">Top 50</option>
        <option :value="100">Top 100</option>
      </select>
      <button @click="fetchData" class="update-btn" :disabled="loading">
        {{ loading ? 'Loading...' : 'Update' }}
      </button>
    </div>
  </div>

  <div class="dashboard-grid">
    <!-- Cross-Asset Volume Flow -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Cross-Asset Volume Flow</h3>
      </div>
      <div class="card-content">
        <div class="volume-flow-container">
          <div class="flow-header">
            <span class="into">Into</span>
            <span class="arrow">←─────────→</span>
            <span class="out-of">Out of</span>
          </div>
          <div class="flow-list">
            <div 
              v-for="coin in volumeFlow" 
              :key="coin.symbol"
              class="flow-item"
            >
              <span class="symbol">{{ coin.symbol }}</span>
              <div class="flow-bar">
                <div 
                  class="bar-fill"
                  :class="coin.netVolumeZScore > 0 ? 'positive' : 'negative'"
                  :style="{ 
                    width: Math.abs(coin.netVolumeZScore) * 20 + '%',
                    marginLeft: coin.netVolumeZScore < 0 ? 'auto' : '0'
                  }"
                ></div>
              </div>
              <span class="zscore" :class="coin.netVolumeZScore > 0 ? 'positive' : 'negative'">
                {{ coin.netVolumeZScore > 0 ? '+' : '' }}{{ coin.netVolumeZScore.toFixed(2) }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
    <!-- Rotation Matrix -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Rotation Matrix</h3>
      </div>
      <div class="card-content">
        <div class="rotation-matrix">
          <div class="matrix-header">Money flow between coins</div>
          <table class="rotation-table" v-if="rotationCoins.length > 0">
            <thead>
              <tr>
                <th>From\To</th>
                <th v-for="coin in rotationCoins" :key="coin">{{ coin }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="fromCoin in rotationCoins" :key="fromCoin">
                <td class="coin-label">{{ fromCoin }}</td>
                <td v-for="toCoin in rotationCoins" :key="toCoin">
                  <span v-if="fromCoin === toCoin" class="diagonal">─</span>
                  <span v-else :class="getRotationClass(fromCoin, toCoin)">
                    {{ getRotationSymbol(fromCoin, toCoin) }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
          <div v-else class="no-data">No rotation data available</div>
          <div class="rotation-summary">
            <span :class="netRotationClass">{{ netRotationText }}</span>
          </div>
        </div>
      </div>
    </div>


    <!-- Liquidity Concentration -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Liquidity Concentration</h3>
      </div>
      <div class="card-content">
        <div class="liquidity-concentration">
          <div class="concentration-header">% of total volume</div>
          <div class="concentration-bars">
            <div 
              v-for="range in liquidityConcentration" 
              :key="range.range"
              class="concentration-item"
            >
              <span class="range-label">{{ range.range }}:</span>
              <div class="bar-container">
                <div 
                  class="bar"
                  :style="{ width: range.percentage + '%' }"
                ></div>
              </div>
              <span class="percentage">{{ range.percentage.toFixed(0) }}%</span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Hourly Seasonality -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Hourly Seasonality</h3>
      </div>
      <div class="card-content">
        <div class="seasonality-container">
          <div class="seasonality-header">Z-Score by Hour (UTC)</div>
          <div class="chart-container">
            <TimeSeriesChart
              v-if="hourlySeasonalityData.length > 0"
              :data="hourlySeasonalityData"
              y-field="avgZScore"
              :show-grid="false"
              :show-labels="true"
            />
          </div>
          <div class="seasonality-stats">
            <span class="peak">Peak: {{ peakHour }}:00 UTC</span>
            <span class="trough">Trough: {{ troughHour }}:00 UTC</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Day of Week Effect -->
    <div class="metric-card">
      <div class="card-header">
        <h3>Day-of-Week Effect</h3>
      </div>
      <div class="card-content">
        <div class="day-effect-container">
          <div class="day-effect-header">Average Z-Score</div>
          <div class="day-bars">
            <div 
              v-for="day in dayOfWeekEffect" 
              :key="day.day"
              class="day-item"
            >
              <div class="bar-wrapper">
                <div 
                  class="day-bar"
                  :class="day.avgZScore > 0 ? 'positive' : 'negative'"
                  :style="{ 
                    height: Math.abs(day.avgZScore) * 50 + 'px',
                    marginTop: day.avgZScore > 0 ? 'auto' : '0'
                  }"
                ></div>
              </div>
              <span class="day-label">{{ day.day }}</span>
              <span class="day-value">{{ day.avgZScore.toFixed(2) }}</span>
            </div>
          </div>
          <div class="day-scale">
            <span class="scale-min">-0.5</span>
            <span class="scale-zero">0</span>
            <span class="scale-max">+0.5</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'

import { useMarketMicrostructure } from '@/composables/useMarketMicrostructure' 
import MetricCard from '../components/common/MetricCard.vue' 

import TimeSeriesChart from '../components/charts/TimeSeriesChart.vue'

const {
  loading,
  error,
  timeframe,
  topN,
  period,
  exchange,
  volumeFlow,
  rotationMatrix,
  liquidityConcentration,
  hourlySeasonality,
  dayOfWeekEffect,
  fetchData
} = useMarketMicrostructure()



const rotationCoins = computed(() => {
  const coins = new Set<string>()
  
  // Add all from and to coins from the rotation matrix
  rotationMatrix.value.forEach(item => {
    coins.add(item.from)
    coins.add(item.to)
  })
  
  return Array.from(coins).sort()
})



const getRotationClass = (from: string, to: string) => {
  const flow = rotationMatrix.value.find(r => r.from === from && r.to === to)
  if (!flow) return ''
  return flow.flow > 0 ? 'flow-up' : 'flow-down'
}

const getRotationSymbol = (from: string, to: string) => {
  const flow = rotationMatrix.value.find(r => r.from === from && r.to === to)
  if (!flow) return '○'
  return flow.flow > 0 ? '▲' : '▼'
}



const netRotationClass = computed(() => {
  const totalFlow = rotationMatrix.value.reduce((sum, r) => sum + r.flow, 0)
  return totalFlow > 0 ? 'risk-on' : 'risk-off'
})

const netRotationText = computed(() => {
  const totalFlow = rotationMatrix.value.reduce((sum, r) => sum + r.flow, 0)
  return totalFlow > 0 ? 'Net: Risk-On Rotation' : 'Net: Risk-Off Rotation'
})


const hourlySeasonalityData = computed(() => {
  const baseDate = new Date(); // Current date: Sep 04, 2025, 07:28 AM EDT (11:28 UTC)
  baseDate.setUTCHours(0, 0, 0, 0); // Set to start of UTC day

  return hourlySeasonality.value.map(h => ({
    timestamp: new Date(baseDate.getTime() + h.hour * 3600000).getTime(), // Offset by hour in milliseconds
    avgZScore: h.avgZScore
  }));
});


const peakHour = computed(() => {
  if (!hourlySeasonality.value.length) return 0
  const peak = hourlySeasonality.value.reduce((max, h) => 
    h.avgZScore > max.avgZScore ? h : max
  )
  return peak.hour
})

const troughHour = computed(() => {
  if (!hourlySeasonality.value.length) return 0
  const trough = hourlySeasonality.value.reduce((min, h) => 
    h.avgZScore < min.avgZScore ? h : min
  )
  return trough.hour
})

onMounted(() => {
  fetchData()
})
</script>

