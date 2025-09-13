<template>
  <div class="signal-performance-dashboard">
    <!-- Header -->
    <div class="dashboard-header">
      <h2>Signal Performance & Risk Analytics</h2>
      <div class="header-controls">
        <select v-model="selectedTimeframe" class="timeframe-select">
          <option value="1d">1 Day Forward</option>
          <option value="5d">5 Day Forward</option>
          <option value="10d">10 Day Forward</option>
          <option value="20d">20 Day Forward</option>
        </select>
      </div>
    </div>

    <!-- 2x2 Grid + Bottom Panel -->
    <div class="performance-grid">
      <!-- Row 1: Signal-Return Scatter | Trend Decomposition -->
      <div class="grid-item signal-return-panel">
        <h3>Signal-Return Relationship</h3>
        <div class="panel-content">
          <!-- Scatter plot of combined signal vs forward returns -->
          <div class="scatter-container">
            <div class="scatter-chart-placeholder">
              <div class="chart-title">Combined Signal vs {{ selectedTimeframe }} Forward Returns</div>
              <div class="scatter-points">
                <div 
                  v-for="(point, i) in scatterData" 
                  :key="i"
                  class="scatter-point"
                  :style="{
                    left: (point.signal + 3) * 16.67 + '%',
                    top: (100 - (point.return + 10) * 5) + '%',
                    backgroundColor: point.return > 0 ? '#00BF63' : '#FF4757'
                  }"
                  :title="`Signal: ${point.signal.toFixed(2)}, Return: ${point.return.toFixed(2)}%`"
                ></div>
                <!-- Fixed regression line with proper CSS property types -->
                <div 
                  class="regression-line"
                  :style="{
                    transform: 'rotate(15deg)',
                    backgroundColor: '#6366f1',
                    height: '2px',
                    width: '80%',
                    position: 'absolute' as const,
                    top: '50%',
                    left: '10%'
                  }"
                ></div>
              </div>
            </div>
          </div>
          
          <!-- Performance metrics -->
          <div class="performance-metrics">
            <div class="metric-item">
              <span class="label">R²:</span>
              <span class="value">{{ rSquared.toFixed(3) }}</span>
            </div>
            <div class="metric-item">
              <span class="label">IC:</span>
              <span class="value" :class="icClass">{{ informationCoef.toFixed(3) }}</span>
            </div>
            <div class="metric-item">
              <span class="label">Hit Rate:</span>
              <span class="value">{{ hitRate.toFixed(1) }}%</span>
            </div>
          </div>
        </div>
      </div>

      <div class="grid-item trend-decomp-panel">
        <h3>Trend Decomposition</h3>
        <div class="panel-content">
          <!-- Trend components chart -->
          <div class="trend-components">
            <div class="component-chart" v-for="component in trendComponents" :key="component.name">
              <div class="component-header">
                <span class="component-name">{{ component.name }}</span>
                <span class="component-value" :style="{ color: component.color }">
                  {{ component.current.toFixed(3) }}
                </span>
              </div>
              <div class="component-sparkline">
                <div 
                  v-for="(val, i) in component.history" 
                  :key="i"
                  class="sparkline-bar"
                  :style="{
                    height: Math.abs(val * 50) + 'px',
                    backgroundColor: component.color,
                    opacity: 0.7 + (i / component.history.length) * 0.3
                  }"
                ></div>
              </div>
            </div>
          </div>
          
          <!-- Aggregated trend indicator -->
          <div class="agg-trend-indicator">
            <span class="label">Aggregated Trend:</span>
            <div class="trend-gauge">
              <div 
                class="gauge-fill" 
                :style="{ 
                  width: Math.abs(aggregatedTrend * 50) + '%',
                  backgroundColor: aggregatedTrend > 0 ? '#00BF63' : '#FF4757'
                }"
              ></div>
            </div>
            <span class="trend-value">{{ aggregatedTrend.toFixed(3) }}</span>
          </div>
        </div>
      </div>

      <!-- Row 2: Risk Heatmaps | Volatility Analysis -->
      <div class="grid-item risk-heatmap-panel">
        <h3>Risk Heatmaps</h3>
        <div class="panel-content">
          <!-- Returns heatmap -->
          <div class="heatmap-container">
            <div class="heatmap-title">Daily Returns & Expected Returns</div>
            <div class="heatmap-grid">
              <div 
                v-for="(cell, i) in returnsHeatmap" 
                :key="i"
                class="heatmap-cell"
                :style="{ backgroundColor: getHeatmapColor(cell.value) }"
                :title="`${cell.asset}: ${cell.value.toFixed(2)}%`"
              >
                <span class="cell-label">{{ cell.asset }}</span>
                <span class="cell-value">{{ cell.value.toFixed(1) }}%</span>
              </div>
            </div>
          </div>
          
          <!-- Volatility highlight -->
          <div class="vol-highlight">
            <span class="label">Highlighted by Annualized Vol</span>
            <div class="vol-legend">
              <div class="legend-item"><div class="color-box low-vol"></div> Low (&lt;50%)</div>
              <div class="legend-item"><div class="color-box med-vol"></div> Medium (50-100%)</div>
              <div class="legend-item"><div class="color-box high-vol"></div> High (&gt;100%)</div>
            </div>
          </div>
        </div>
      </div>

      <div class="grid-item volatility-panel">
        <h3>Volatility Analysis</h3>
        <div class="panel-content">
          <!-- Long vs Short term vol -->
          <div class="vol-comparison">
            <div class="vol-chart-container">
              <div class="vol-chart-title">Long vs Short Term Realized Vol</div>
              <div class="vol-lines">
                <div class="vol-line short-term">
                  <span class="line-label">Short (5d): {{ shortTermVol.toFixed(2) }}%</span>
                  <div class="line-chart">
                    <div 
                      v-for="(point, i) in shortTermVolHistory" 
                      :key="i"
                      class="vol-point"
                      :style="{ height: point * 100 + 'px', backgroundColor: '#00D4FF' }"
                    ></div>
                  </div>
                </div>
                <div class="vol-line long-term">
                  <span class="line-label">Long (20d): {{ longTermVol.toFixed(2) }}%</span>
                  <div class="line-chart">
                    <div 
                      v-for="(point, i) in longTermVolHistory" 
                      :key="i"
                      class="vol-point"
                      :style="{ height: point * 100 + 'px', backgroundColor: '#FF6B6B' }"
                    ></div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Vol forecast vs actual -->
          <div class="vol-forecast">
            <div class="forecast-metric">
              <span class="label">Vol Forecast vs Actual:</span>
              <span class="value" :class="forecastAccuracyClass">
                {{ forecastAccuracy.toFixed(1) }}% accuracy
              </span>
            </div>
            <div class="regime-indicator">
              <span class="label">Vol Regime:</span>
              <span class="regime-badge" :class="volRegimeClass">{{ volRegime }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Bottom Panel: Target Exposure Heatmap -->
      <div class="grid-item-full exposure-heatmap-panel">
        <h3>Target Exposure Heatmap</h3>
        <div class="panel-content">
          <div class="exposure-grid">
            <div 
              v-for="(exposure, i) in targetExposures" 
              :key="i"
              class="exposure-item"
              :style="{ 
                width: Math.abs(exposure.weight) * 200 + 'px',
                backgroundColor: exposure.weight > 0 ? '#00BF63' : '#FF4757',
                opacity: 0.7 + Math.abs(exposure.weight) * 0.3
              }"
            >
              <div class="exposure-label">{{ exposure.asset }}</div>
              <div class="exposure-weight">{{ (exposure.weight * 100).toFixed(1) }}%</div>
              <div class="exposure-vol">Vol: {{ (exposure.vol * 100).toFixed(0) }}%</div>
            </div>
          </div>
          
          <!-- Long/Short breakdown -->
          <div class="exposure-breakdown">
            <div class="breakdown-item long">
              <span class="label">Total Long:</span>
              <span class="value">{{ totalLongExposure.toFixed(1) }}%</span>
            </div>
            <div class="breakdown-item short">
              <span class="label">Total Short:</span>
              <span class="value">{{ totalShortExposure.toFixed(1) }}%</span>
            </div>
            <div class="breakdown-item net">
              <span class="label">Net Exposure:</span>
              <span class="value">{{ netExposure.toFixed(1) }}%</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, inject } from 'vue'

// Inject shared state (with fallback for demo)
const signalData = inject('signalData', ref({}))
const marketData = inject('marketData', ref({}))

// Local state
const selectedTimeframe = ref('5d')

// Mock data for visualization
const scatterData = computed(() => 
  Array.from({ length: 50 }, () => ({
    signal: (Math.random() - 0.5) * 6,
    return: (Math.random() - 0.5) * 20
  }))
)

const rSquared = computed(() => 0.234)
const informationCoef = computed(() => 0.156)
const hitRate = computed(() => 58.3)

const icClass = computed(() => informationCoef.value > 0.1 ? 'positive' : 'negative')

// Trend decomposition components
const trendComponents = computed(() => [
  {
    name: 'Systematic Trend',
    color: '#00D4FF',
    current: 0.023,
    history: Array.from({ length: 20 }, () => (Math.random() - 0.5) * 0.1)
  },
  {
    name: 'Idiosyncratic Trend', 
    color: '#FF6B6B',
    current: -0.012,
    history: Array.from({ length: 20 }, () => (Math.random() - 0.5) * 0.08)
  },
  {
    name: 'Reversion Component',
    color: '#00BF63',
    current: 0.008,
    history: Array.from({ length: 20 }, () => (Math.random() - 0.5) * 0.06)
  }
])

const aggregatedTrend = computed(() => 
  trendComponents.value.reduce((sum, comp) => sum + comp.current, 0)
)

// Returns heatmap data
const returnsHeatmap = computed(() => [
  { asset: 'BTC', value: 2.3, vol: 0.65 },
  { asset: 'ETH', value: 1.8, vol: 0.72 },
  { asset: 'SOL', value: -0.5, vol: 1.15 },
  { asset: 'ADA', value: 0.9, vol: 0.98 },
  { asset: 'DOT', value: -1.2, vol: 1.05 },
  { asset: 'AVAX', value: 1.4, vol: 1.25 }
])

const getHeatmapColor = (value: number): string => {
  if (value > 1) return '#00BF63'
  if (value > 0) return '#2ED573'
  if (value > -1) return '#FFA502'
  return '#FF4757'
}

// Volatility metrics
const shortTermVol = computed(() => 0.034)
const longTermVol = computed(() => 0.028)

const shortTermVolHistory = computed(() => 
  Array.from({ length: 15 }, () => Math.random() * 0.05 + 0.02)
)
const longTermVolHistory = computed(() => 
  Array.from({ length: 15 }, () => Math.random() * 0.04 + 0.02)
)

const forecastAccuracy = computed(() => 76.4)
const forecastAccuracyClass = computed(() => 
  forecastAccuracy.value > 70 ? 'good-forecast' : 'poor-forecast'
)

const volRegime = computed(() => 'Medium')
const volRegimeClass = computed(() => `regime-${volRegime.value.toLowerCase()}`)

// Target exposures
const targetExposures = computed(() => [
  { asset: 'BTC', weight: 0.042, vol: 0.65 },
  { asset: 'ETH', weight: 0.038, vol: 0.72 },
  { asset: 'SOL', weight: 0.021, vol: 1.15 },
  { asset: 'ADA', weight: -0.015, vol: 0.98 },
  { asset: 'DOGE', weight: -0.021, vol: 1.80 },
  { asset: 'SHIB', weight: -0.012, vol: 2.20 }
])

const totalLongExposure = computed(() => 
  targetExposures.value
    .filter(e => e.weight > 0)
    .reduce((sum, e) => sum + e.weight, 0) * 100
)

const totalShortExposure = computed(() => 
  Math.abs(targetExposures.value
    .filter(e => e.weight < 0)
    .reduce((sum, e) => sum + e.weight, 0)) * 100
)

const netExposure = computed(() => 
  targetExposures.value.reduce((sum, e) => sum + e.weight, 0) * 100
)
</script>
