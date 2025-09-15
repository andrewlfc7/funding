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
                    left: Math.max(0, Math.min(100, (point.signal + 3) * 16.67)) + '%',
                    top: Math.max(0, Math.min(100, 100 - (point.return + 10) * 5)) + '%',
                    backgroundColor: point.return > 0 ? '#00BF63' : '#FF4757'
                  }"
                  :title="`Signal: ${point.signal.toFixed(2)}, Return: ${point.return.toFixed(2)}%`"
                ></div>
                <!-- Regression line -->
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
          
          <!-- Performance metrics - FIXED -->
          <div class="performance-metrics">
            <div class="metric-item">
              <span class="label">R²</span>
              <span class="value">{{ rSquared.toFixed(3) }}</span>
            </div>
            <div class="metric-item">
              <span class="label">IC</span>
              <span class="value" :class="icClass">{{ informationCoef.toFixed(3) }}</span>
            </div>
            <div class="metric-item">
              <span class="label">Hit Rate</span>
              <span class="value">{{ hitRate.toFixed(1) }}%</span>
            </div>
          </div>
        </div>
      </div>

      <div class="grid-item trend-decomp-panel">
        <h3>Trend Decomposition</h3>
        <div class="panel-content">
          <!-- Use TimeSeriesChart for trend components with proper container -->
          <div class="trend-timeseries-container">
            <TimeSeriesChart
              :series="trendDecompositionSeries"
              :height="220"
              y-label="Signal Strength"
              :y-format="(value) => value.toFixed(3)"
              :show-threshold-lines="true"
              :thresholds="[0]"
              :threshold-labels="['Zero Line']"
            />
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
                :style="{ 
                  backgroundColor: getHeatmapColor(cell.value),
                  borderColor: getVolatilityBorderColor(cell.vol)
                }"
                :title="`${cell.asset}: ${cell.value.toFixed(2)}% return, ${(cell.vol * 100).toFixed(0)}% vol`"
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
          <!-- Use TimeSeriesChart for volatility analysis with proper container -->
          <div class="vol-timeseries-container">
            <TimeSeriesChart
              :series="volatilitySeries"
              :height="180"
              y-label="Realized Volatility"
              :y-format="(value) => `${(value * 100).toFixed(1)}%`"
            />
          </div>
          
          <!-- Vol forecast vs actual with proper metrics -->
          <div class="vol-forecast">
            <div class="forecast-metric">
              <span class="label">Vol Forecast vs Actual</span>
              <span class="value" :class="forecastAccuracyClass">
                {{ forecastAccuracy.toFixed(1) }}%
              </span>
            </div>
            <div class="regime-indicator">
              <span class="label">Vol Regime</span>
              <span class="regime-badge" :class="volRegimeClass">{{ volRegime }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Bottom Panel: Target Exposure Heatmap -->
      <div class="grid-item-full exposure-heatmap-panel">
        <h3>Target Exposure Heatmap</h3>
        <div class="panel-content">
          <!-- Custom exposure heatmap without ExposureHeatmap component -->
          <div class="exposure-heatmap-container">
            <div class="heatmap-header">
              <h4>Target Exposure Distribution</h4>
              <div class="heatmap-controls">
                <button 
                  v-for="mode in exposureModes" 
                  :key="mode.value"
                  :class="['mode-btn', { active: selectedExposureMode === mode.value }]"
                  @click="selectedExposureMode = mode.value"
                >
                  {{ mode.label }}
                </button>
              </div>
            </div>
            
            <div class="heatmap-content">
              <!-- Position grid -->
              <div class="heatmap-grid exposure-grid">
                <div 
                  v-for="(position, i) in targetExposures" 
                  :key="i"
                  :class="[
                    'position-cell',
                    position.weight > 0 ? 'long-position' : 'short-position',
                    Math.abs(position.weight) > 0.03 ? 'large-position' : ''
                  ]"
                  :title="`${position.asset}: ${(position.weight * 100).toFixed(2)}% weight`"
                >
                  <div class="cell-header">
                    <span class="asset-symbol">{{ position.asset }}</span>
                    <span class="position-size">{{ (position.weight * 100).toFixed(1) }}%</span>
                  </div>
                  
                  <div class="weight-bar">
                    <div 
                      class="weight-fill"
                      :style="{
                        width: Math.min(100, (Math.abs(position.weight) * 100 / 0.05 * 100)) + '%',
                        backgroundColor: position.weight > 0 ? '#00BF63' : '#FF4757'
                      }"
                    ></div>
                  </div>
                  
                  <div class="cell-metrics">
                    <div class="metric-item exposure-metric" v-if="selectedExposureMode === 'weight'">
                      <span class="metric-label">Signal</span>
                      <span class="metric-value" :class="position.signal > 0 ? 'positive' : 'negative'">
                        {{ position.signal.toFixed(2) }}
                      </span>
                    </div>
                    <div class="metric-item exposure-metric" v-else-if="selectedExposureMode === 'volatility'">
                      <span class="metric-label">Vol</span>
                      <span class="metric-value">{{ (position.volatility * 100).toFixed(0) }}%</span>
                    </div>
                    <div class="metric-item exposure-metric" v-else-if="selectedExposureMode === 'risk'">
                      <span class="metric-label">Risk Contrib</span>
                      <span class="metric-value">{{ (position.risk_contribution * 100).toFixed(1) }}%</span>
                    </div>
                    <div class="metric-item exposure-metric" v-else-if="selectedExposureMode === 'pnl'">
                      <span class="metric-label">PnL</span>
                      <span class="metric-value" :class="position.pnl > 0 ? 'positive' : 'negative'">
                        {{ (position.pnl * 100).toFixed(1) }}%
                      </span>
                    </div>
                    
                    <div class="metric-item exposure-metric">
                      <span class="metric-label">Expected Return</span>
                      <span class="metric-value" :class="position.expected_return > 0 ? 'positive' : 'negative'">
                        {{ (position.expected_return * 100).toFixed(1) }}%
                      </span>
                    </div>
                  </div>
                </div>
              </div>
              
              <!-- Summary section -->
              <div class="exposure-summary">
                <div class="summary-row">
                  <div class="summary-item long">
                    <span class="summary-label">Total Long</span>
                    <span class="summary-value">{{ totalLongExposure.toFixed(1) }}%</span>
                  </div>
                  <div class="summary-item short">
                    <span class="summary-label">Total Short</span>
                    <span class="summary-value">{{ totalShortExposure.toFixed(1) }}%</span>
                  </div>
                  <div class="summary-item net">
                    <span class="summary-label">Net</span>
                    <span class="summary-value">{{ netExposure.toFixed(1) }}%</span>
                  </div>
                  <div class="summary-item gross">
                    <span class="summary-label">Gross</span>
                    <span class="summary-value">{{ grossExposure.toFixed(1) }}%</span>
                  </div>
                </div>
                
                <div class="risk-metrics">
                  <div class="risk-item">
                    <span class="risk-label">Portfolio Vol</span>
                    <span class="risk-value">{{ portfolioVol.toFixed(0) }}%</span>
                  </div>
                  <div class="risk-item">
                    <span class="risk-label">VaR (95%)</span>
                    <span class="risk-value">{{ valueAtRisk.toFixed(1) }}%</span>
                  </div>
                  <div class="risk-item">
                    <span class="risk-label">Concentration</span>
                    <span class="risk-value">{{ concentrationRatio.toFixed(0) }}%</span>
                    <div class="concentration-bar">
                      <div 
                        class="concentration-fill"
                        :style="{
                          width: Math.min(100, concentrationRatio) + '%',
                          backgroundColor: concentrationRatio > 70 ? '#FF4757' : concentrationRatio > 50 ? '#FFA502' : '#00BF63'
                        }"
                      ></div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, inject } from 'vue'
import TimeSeriesChart from '@/components/zscore/components/charts/TimeSeriesChart.vue'

// Inject shared state (with fallback for demo)
const signalData = inject('signalData', ref({}))
const marketData = inject('marketData', ref({}))

// Local state
const selectedTimeframe = ref('5d')
const selectedExposureMode = ref('weight')

// Exposure mode options
const exposureModes = [
  { value: 'weight', label: 'Weight' },
  { value: 'volatility', label: 'Volatility' },
  { value: 'risk', label: 'Risk' },
  { value: 'pnl', label: 'P&L' }
]

// Mock data for visualization with proper bounds
const scatterData = computed(() => 
  Array.from({ length: 50 }, () => ({
    signal: Math.max(-3, Math.min(3, (Math.random() - 0.5) * 6)),
    return: Math.max(-10, Math.min(10, (Math.random() - 0.5) * 20))
  }))
)

// Performance metrics with realistic values
const rSquared = computed(() => 0.234)
const informationCoef = computed(() => 0.156)
const hitRate = computed(() => 58.3)

const icClass = computed(() => informationCoef.value > 0.1 ? 'positive' : 'negative')

// Generate mock time series data for trend decomposition with better time distribution
const generateTimeSeriesData = (length: number, baseValue: number, volatility: number) => {
  const now = Date.now()
  const data = []
  let currentValue = baseValue
  
  for (let i = 0; i < length; i++) {
    const change = (Math.random() - 0.5) * volatility
    currentValue += change
    // Add some mean reversion
    currentValue = currentValue * 0.95 + baseValue * 0.05
    
    data.push({
      timestamp: now - (length - i) * 3600000, // Hourly data going back
      value: Number(currentValue.toFixed(4))
    })
  }
  
  return data
}

// Trend decomposition data for TimeSeriesChart
const trendDecompositionSeries = computed(() => [
  {
    symbol: 'Systematic Trend',
    data: generateTimeSeriesData(100, 0.023, 0.02)
  },
  {
    symbol: 'Idiosyncratic Trend', 
    data: generateTimeSeriesData(100, -0.012, 0.015)
  },
  {
    symbol: 'Reversion Component',
    data: generateTimeSeriesData(100, 0.008, 0.01)
  }
])

const aggregatedTrend = computed(() => {
  const sum = trendDecompositionSeries.value.reduce((total, series) => {
    const lastValue = series.data[series.data.length - 1]?.value || 0
    return total + lastValue
  }, 0)
  return Number(sum.toFixed(4))
})

// Volatility data for TimeSeriesChart with realistic values
const volatilitySeries = computed(() => [
  {
    symbol: 'Short Term (5d)',
    data: generateTimeSeriesData(100, 0.034, 0.005).map(d => ({
      ...d,
      value: Math.max(0.01, d.value) // Ensure positive volatility
    }))
  },
  {
    symbol: 'Long Term (20d)',
    data: generateTimeSeriesData(100, 0.028, 0.003).map(d => ({
      ...d,
      value: Math.max(0.01, d.value) // Ensure positive volatility
    }))
  }
])

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
  const intensity = Math.min(255, Math.abs(value) * 50)
  if (value > 1.5) return `rgba(0, 191, 99, 0.9)`
  if (value > 0.5) return `rgba(46, 213, 115, 0.7)`
  if (value > -0.5) return `rgba(255, 165, 2, 0.7)`
  if (value > -1.5) return `rgba(255, 107, 74, 0.7)`
  return `rgba(255, 71, 87, 0.9)`
}

const getVolatilityBorderColor = (vol: number): string => {
  if (vol > 1.0) return '#FF4757'  // High vol - red border
  if (vol > 0.5) return '#FFA502'  // Medium vol - orange border
  return '#00BF63'  // Low vol - green border
}

const forecastAccuracy = computed(() => 76.4)
const forecastAccuracyClass = computed(() => 
  forecastAccuracy.value > 70 ? 'good-forecast' : 'poor-forecast'
)

const volRegime = computed(() => 'Medium')
const volRegimeClass = computed(() => `regime-${volRegime.value.toLowerCase()}`)

// Target exposures for custom heatmap with better data distribution
const targetExposures = computed(() => [
  { 
    asset: 'BTC', 
    weight: 0.042, 
    volatility: 0.65, 
    signal: 1.2, 
    expected_return: 0.08, 
    risk_contribution: 0.032,
    pnl: 0.024
  },
  { 
    asset: 'ETH', 
    weight: 0.038, 
    volatility: 0.72, 
    signal: 0.9, 
    expected_return: 0.06, 
    risk_contribution: 0.028,
    pnl: 0.018
  },
  { 
    asset: 'SOL', 
    weight: 0.021, 
    volatility: 1.15, 
    signal: 0.5, 
    expected_return: 0.04, 
    risk_contribution: 0.025,
    pnl: 0.012
  },
  { 
    asset: 'ADA', 
    weight: -0.015, 
    volatility: 0.98, 
    signal: -0.6, 
    expected_return: -0.03, 
    risk_contribution: 0.015,
    pnl: -0.008
  },
  { 
    asset: 'DOGE', 
    weight: -0.021, 
    volatility: 1.80, 
    signal: -1.1, 
    expected_return: -0.05, 
    risk_contribution: 0.038,
    pnl: -0.015
  },
  { 
    asset: 'SHIB', 
    weight: -0.012, 
    volatility: 2.20, 
    signal: -0.8, 
    expected_return: -0.02, 
    risk_contribution: 0.026,
    pnl: -0.005
  },
  { 
    asset: 'AVAX', 
    weight: 0.028, 
    volatility: 1.25, 
    signal: 0.7, 
    expected_return: 0.05, 
    risk_contribution: 0.022,
    pnl: 0.014
  },
  { 
    asset: 'DOT', 
    weight: -0.018, 
    volatility: 1.05, 
    signal: -0.4, 
    expected_return: -0.025, 
    risk_contribution: 0.019,
    pnl: -0.009
  }
])

// Portfolio summary calculations
const totalLongExposure = computed(() => 
  targetExposures.value
    .filter(p => p.weight > 0)
    .reduce((sum, p) => sum + p.weight, 0) * 100
)

const totalShortExposure = computed(() => 
  Math.abs(targetExposures.value
    .filter(p => p.weight < 0)
    .reduce((sum, p) => sum + p.weight, 0)) * 100
)

const netExposure = computed(() => 
  targetExposures.value.reduce((sum, p) => sum + p.weight, 0) * 100
)

const grossExposure = computed(() => 
  targetExposures.value.reduce((sum, p) => sum + Math.abs(p.weight), 0) * 100
)

const portfolioVol = computed(() => 7)
const valueAtRisk = computed(() => 9.1)
const concentrationRatio = computed(() => Math.min(100, 84))
</script>