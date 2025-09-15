<template>
  <div class="trend-direction-dashboard">
    <!-- Header -->
    <div class="dashboard-header">
      <h2>Portfolio Exposure</h2>
      <div class="trend-summary">
        <div class="trend-indicator" :class="trendStateClass">
          <span class="trend-label">Current Trend:</span>
          <span class="trend-value">{{ trendState }}</span>
        </div>
        <div class="trend-strength">
          <span class="strength-label">Strength:</span>
          <div class="strength-bars">
            <div 
              v-for="i in 5" 
              :key="i"
              class="strength-bar"
              :class="{ active: i <= trendStrength }"
            ></div>
          </div>
        </div>
      </div>
    </div>

    <!-- 2x3 Grid Layout -->
    <div class="direction-grid">
      <!-- Row 1: Portfolio Trend State | Long/Short Breakdown -->
      <div class="grid-item trend-state-panel">
        <h3>Portfolio Trend State</h3>
        <div class="panel-content">
          <!-- Trend gauge -->
          <div class="trend-gauge-container">
            <div class="trend-gauge">
              <div class="gauge-arc">
                <div 
                  class="gauge-needle"
                  :style="{ transform: `rotate(${trendAngle}deg)` }"
                ></div>
              </div>
              <div class="gauge-labels">
                <span class="gauge-label bearish">Bearish</span>
                <span class="gauge-label neutral">Neutral</span>
                <span class="gauge-label bullish">Bullish</span>
              </div>
            </div>
            <div class="gauge-reading">
              <div class="reading-value">{{ trendReading.toFixed(1) }}</div>
              <div class="reading-label">Trend Score</div>
            </div>
          </div>
          
          <!-- Trend duration and regime -->
          <div class="trend-metrics">
            <div class="metric-item">
              <span class="metric-label">Duration:</span>
              <span class="metric-value">{{ trendDuration }} days</span>
            </div>
            <div class="metric-item">
              <span class="metric-label">Market Regime:</span>
              <span class="metric-value regime" :class="regimeClass">{{ marketRegime }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="grid-item long-short-panel">
        <h3>Long/Short Breakdown</h3>
        <div class="panel-content">
          <!-- Net L/S Position Chart with Zero Line -->
          <div class="net-position-chart">
            <div class="chart-header">
              <div class="chart-title">Net Position Timeline</div>
              <div class="current-net">
                <span class="net-label">Current Net:</span>
                <span class="net-value" :class="currentNetExposure >= 0 ? 'positive' : 'negative'">
                  {{ currentNetExposure >= 0 ? '+' : '' }}{{ currentNetExposure.toFixed(1) }}%
                </span>
              </div>
            </div>
            
            <div class="net-chart-container">
              <!-- Zero line -->
              <div class="zero-line"></div>
              
              <!-- Net position bars -->
              <div class="net-timeline">
                <div 
                  v-for="(netPos, i) in netPositionHistory" 
                  :key="i"
                  class="net-bar"
                  :class="{ 
                    'net-long': netPos > 0, 
                    'net-short': netPos < 0,
                    'net-neutral': netPos === 0
                  }"
                  :style="getNetBarStyle(netPos)"
                  :title="`Day ${i + 1}: ${netPos > 0 ? '+' : ''}${netPos.toFixed(1)}% net`"
                ></div>
              </div>
              
              <!-- Chart labels -->
              <div class="chart-labels">
                <span class="label-short">Net Short</span>
                <span class="label-neutral">Neutral</span>
                <span class="label-long">Net Long</span>
              </div>
            </div>
          </div>

          <!-- L/S ratio and performance -->
          <div class="ls-details">
            <div class="ratio-section">
              <div class="ratio-header">
                <span class="ratio-label">L/S Ratio:</span>
                <span class="ratio-value">{{ lsRatio.toFixed(2) }}</span>
              </div>
              <div class="ratio-breakdown">
                <div class="breakdown-item long">
                  <span class="breakdown-label">Long Allocation:</span>
                  <span class="breakdown-value">{{ longAllocation.toFixed(1) }}%</span>
                </div>
                <div class="breakdown-item short">
                  <span class="breakdown-label">Short Allocation:</span>
                  <span class="breakdown-value">{{ shortAllocation.toFixed(1) }}%</span>
                </div>
              </div>
            </div>

            <!-- Long vs Short performance -->
            <div class="ls-performance">
              <div class="perf-item long-perf">
                <div class="perf-label">Long Positions Trend</div>
                <div class="perf-indicator">
                  <div class="perf-arrow" :class="longTrendDirection">{{ longTrendArrow }}</div>
                  <div class="perf-value positive">{{ longTrendStrength.toFixed(1) }}%</div>
                </div>
              </div>
              <div class="perf-item short-perf">
                <div class="perf-label">Short Positions Trend</div>
                <div class="perf-indicator">
                  <div class="perf-arrow" :class="shortTrendDirection">{{ shortTrendArrow }}</div>
                  <div class="perf-value negative">{{ shortTrendStrength.toFixed(1) }}%</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Row 2: Directional Exposure | Trend Persistence -->
      <div class="grid-item directional-exposure-panel">
        <h3>Directional Exposure</h3>
        <div class="panel-content">
          <!-- Net exposure over time -->
          <div class="exposure-chart">
            <div class="exposure-timeline">
            <div 
              v-for="(exposure, i) in netExposureHistory" 
              :key="i"
              class="exposure-bar"
              :style="{ 
                height: Math.abs(exposure) * 100 + 'px',
                backgroundColor: exposure > 0 ? '#00BF63' : (exposure < 0 ? '#FF4757' : '#6B7280'),
                transform: exposure < 0 ? 'translateY(50%)' : 'translateY(-50%)'
              }"
              :title="`Day ${i + 1}: ${(exposure * 100).toFixed(1)}%`"
            ></div>
            </div>
            <div class="exposure-zero-line"></div>
          </div>
        </div>
      </div>
      
      <div class="grid-item trend-persistence-panel">
        <h3>Trend Persistence</h3>
        <div class="panel-content">
          <!-- Trend duration analysis -->
          <div class="duration-analysis">
            <div class="duration-chart">
              <div class="chart-title">Trend Duration Distribution</div>
              <div class="duration-bars">
                <div 
                  v-for="(duration, i) in trendDurations" 
                  :key="i"
                  class="duration-bar"
                  :title="`${duration.days} days: ${(duration.frequency * 100).toFixed(1)}%`"
                >
                  <div 
                    class="duration-bar-fill"
                    :style="{ height: duration.frequency * 60 + 'px' }"
                  ></div>
                  <div class="duration-label">{{ duration.days }}d</div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Reversal probability -->
          <div class="reversal-analysis">
            <div class="reversal-gauge">
              <span class="reversal-label">Reversal Probability:</span>
              <div class="reversal-meter">
                <div 
                  class="reversal-fill"
                  :style="{ 
                    width: reversalProbability + '%',
                    backgroundColor: getReversalColor(reversalProbability)
                  }"
                ></div>
              </div>
              <span class="reversal-value">{{ reversalProbability }}%</span>
            </div>
            
            <div class="momentum-sustainability">
              <span class="sustain-label">Momentum Sustainability:</span>
              <div class="sustainability-indicator" :class="sustainabilityClass">
                {{ sustainabilityLevel }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Row 3: Market Regime Timing | Position Concentration -->
      <div class="grid-item regime-timing-panel">
        <h3>Market Regime Timing</h3>
        <div class="panel-content">
          <!-- Regime classification -->
          <div class="regime-classification">
            <div class="regime-probabilities">
              <div 
                v-for="regime in regimeProbabilities" 
                :key="regime.name"
                class="regime-prob-item"
              >
                <div class="regime-name">{{ regime.name }}</div>
                <div class="regime-prob-bar">
                  <div 
                    class="regime-prob-fill"
                    :style="{ 
                      width: regime.probability + '%',
                      backgroundColor: regime.color
                    }"
                  ></div>
                </div>
                <div class="regime-prob-value">{{ regime.probability }}%</div>
              </div>
            </div>
          </div>
          
          <!-- Historical regime performance -->
          <div class="regime-performance">
            <h4>Historical Regime P&L</h4>
            <div class="regime-perf-grid">
              <div 
                v-for="perf in regimePerformance" 
                :key="perf.regime"
                class="regime-perf-item"
              >
                <div class="regime-perf-label">{{ perf.regime }}</div>
                <div class="regime-perf-value" :class="perf.pnl > 0 ? 'positive' : 'negative'">
                  {{ perf.pnl > 0 ? '+' : '' }}{{ perf.pnl.toFixed(1) }}%
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="grid-item position-concentration-panel">
        <h3>Position Concentration</h3>
        <div class="panel-content">
          <!-- Top positions by direction -->
          <div class="top-positions">
            <div class="positions-column">
              <h4>Top 5 Long Positions</h4>
              <div class="position-list">
                <div 
                  v-for="position in top5Long" 
                  :key="position.asset"
                  class="position-row"
                >
                  <span class="pos-asset">{{ position.asset }}</span>
                  <span class="pos-weight">{{ position.weight.toFixed(1) }}%</span>
                  <div class="pos-bar">
                    <div 
                      class="pos-fill long"
                      :style="{ width: position.weight * 20 + 'px' }"
                    ></div>
                  </div>
                </div>
              </div>
            </div>
            
            <div class="positions-column">
              <h4>Top 5 Short Positions</h4>
              <div class="position-list">
                <div 
                  v-for="position in top5Short" 
                  :key="position.asset"
                  class="position-row"
                >
                  <span class="pos-asset">{{ position.asset }}</span>
                  <span class="pos-weight">{{ Math.abs(position.weight).toFixed(1) }}%</span>
                  <div class="pos-bar">
                    <div 
                      class="pos-fill short"
                      :style="{ width: Math.abs(position.weight) * 20 + 'px' }"
                    ></div>
                  </div>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Concentration risk metrics -->
          <div class="concentration-metrics">
            <div class="concentration-item">
              <span class="conc-label">Concentration Risk:</span>
              <div class="risk-meter" :class="concentrationRiskClass">{{ concentrationRisk }}</div>
            </div>
            <div class="concentration-item">
              <span class="conc-label">Diversification Score:</span>
              <div class="diversification-score">{{ diversificationScore.toFixed(1) }}/10</div>
            </div>
            <div class="concentration-item">
              <span class="conc-label">Single Position Risk:</span>
              <div class="single-risk">{{ singlePositionRisk.toFixed(1) }}%</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, inject } from 'vue'

// Inject shared state
const signalData = inject('signalData')!

// Trend state
const trendState = ref('Bull Market')
const trendStrength = ref(4) // 1-5 scale
const trendReading = ref(2.3) // -3 to +3
const trendDuration = ref(18)
const marketRegime = ref('Risk On')

const trendStateClass = computed(() => trendState.value.toLowerCase().replace(' ', '-'))
const regimeClass = computed(() => marketRegime.value.toLowerCase().replace(' ', '-'))

// Trend gauge angle (-90 to +90 degrees)
const trendAngle = computed(() => (trendReading.value / 3) * 90)

// Long/Short metrics with proper net calculation
const longAllocation = ref(58.5) // Total long positions %
const shortAllocation = ref(24.8) // Total short positions % (positive number)
const lsRatio = computed(() => longAllocation.value / shortAllocation.value)
const currentNetExposure = computed(() => longAllocation.value - shortAllocation.value)

// Generate realistic net position history (net long/short over time)
const netPositionHistory = ref(Array.from({ length: 20 }, (_, i) => {
  // Create a trending pattern with some volatility
  const trend = -10 + (i * 3.5) + (Math.random() - 0.5) * 8
  return Math.max(-50, Math.min(50, trend)) // Clamp between -50% and +50%
}))

const longTrendStrength = ref(2.4)
const shortTrendStrength = ref(-1.2)
const longTrendDirection = computed(() => longTrendStrength.value > 0 ? 'up' : 'down')
const shortTrendDirection = computed(() => shortTrendStrength.value > 0 ? 'up' : 'down')
const longTrendArrow = computed(() => longTrendDirection.value === 'up' ? '↗' : '↘')
const shortTrendArrow = computed(() => shortTrendDirection.value === 'up' ? '↗' : '↘')

// Fixed bar style function to properly contain bars
const getNetBarStyle = (netPos: number) => {
  const maxHeight = 35
  const height = Math.abs(netPos) * 0.8
  const clampedHeight = Math.min(height, maxHeight)
  
  let backgroundColor = '#6B7280'
  if (netPos > 0) backgroundColor = '#00BF63'
  if (netPos < 0) backgroundColor = '#FF4757'
  
  return {
    height: `${clampedHeight}px`,
    backgroundColor,
    transform: netPos >= 0 ? 'translateY(-50%)' : 'translateY(50%)',
  }
}

const netExposureHistory = ref(Array.from({ length: 30 }, () => (Math.random() - 0.3) * 0.6))

// Trend persistence
const trendDurations = ref([
  { days: 5, frequency: 0.15 },
  { days: 10, frequency: 0.25 },
  { days: 15, frequency: 0.30 },
  { days: 20, frequency: 0.20 },
  { days: 25, frequency: 0.10 }
])

const reversalProbability = ref(12)
const sustainabilityLevel = ref('Medium')
const sustainabilityClass = computed(() => `sustainability-${sustainabilityLevel.value.toLowerCase()}`)

const getReversalColor = (prob: number) => {
  if (prob < 20) return '#00BF63'
  if (prob < 50) return '#FFA502'
  return '#FF4757'
}

// Market regime
const regimeProbabilities = ref([
  { name: 'Bull Market', probability: 73, color: '#00BF63' },
  { name: 'Bear Market', probability: 15, color: '#FF4757' },
  { name: 'Sideways', probability: 12, color: '#FFA502' }
])

const regimePerformance = ref([
  { regime: 'Bull', pnl: 4.2 },
  { regime: 'Bear', pnl: -1.8 },
  { regime: 'Sideways', pnl: 0.3 }
])

// Position concentration
const top5Long = ref([
  { asset: 'BTC', weight: 4.2 },
  { asset: 'ETH', weight: 3.8 },
  { asset: 'SOL', weight: 2.1 },
  { asset: 'ADA', weight: 1.8 },
  { asset: 'DOT', weight: 1.6 }
])

const top5Short = ref([
  { asset: 'DOGE', weight: -2.1 },
  { asset: 'SHIB', weight: -1.5 },
  { asset: 'LTC', weight: -1.2 },
  { asset: 'XRP', weight: -0.9 },
  { asset: 'TRX', weight: -0.7 }
])

const concentrationRisk = ref('Medium')
const concentrationRiskClass = computed(() => `risk-${concentrationRisk.value.toLowerCase()}`)
const diversificationScore = ref(7.2)
const singlePositionRisk = ref(4.2)
</script>