<template>
  <div class="performance-attribution-dashboard">
    <!-- Header -->
    <div class="dashboard-header">
      <h2>Performance Attribution & P&L Analysis</h2>
      <div class="header-controls">
        <select v-model="selectedPeriod" class="period-select">
          <option value="1d">Today</option>
          <option value="1w">This Week</option>
          <option value="1m">This Month</option>
          <option value="3m">3 Months</option>
        </select>
        <div class="total-pnl" :class="totalPnlClass">
          {{ totalPnl > 0 ? '+' : '' }}${{ formatCurrency(totalPnl) }}
        </div>
      </div>
    </div>

    <!-- Attribution Layout -->
    <div class="attribution-layout">
      <!-- Enhanced P&L Attribution Panel -->
      <div class="waterfall-panel">
        <div class="panel-header">
          <h3>P&L Attribution Analysis</h3>
          <div class="chart-toggle">
            <button 
              class="toggle-btn" 
              :class="{ active: selectedView === 'waterfall' }"
              @click="selectedView = 'waterfall'"
            >
              Waterfall
            </button>
            <button 
              class="toggle-btn" 
              :class="{ active: selectedView === 'pie' }"
              @click="selectedView = 'pie'"
            >
              Breakdown
            </button>
          </div>
        </div>
        
        <div class="chart-container">
          <!-- Waterfall Chart -->
          <div v-if="selectedView === 'waterfall'" class="waterfall-chart-wrapper">
            <div class="waterfall-chart">
              <div 
                v-for="(item, i) in waterfallData" 
                :key="i"
                class="waterfall-item"
                :class="[item.type, { 'has-tooltip': hoveredIndex === i }]"
                @mouseenter="hoveredIndex = i"
                @mouseleave="hoveredIndex = -1"
              >
                <div 
                  class="waterfall-bar"
                  :style="{ 
                    height: Math.max(Math.abs(item.value) * scaleFactor, 8) + 'px',
                  }"
                  :data-value="item.value"
                ></div>
                <div class="waterfall-label">{{ item.label }}</div>
                <div class="waterfall-value">
                  {{ item.value > 0 ? '+' : '' }}${{ formatCurrency(item.value) }}
                </div>
                
                <!-- Tooltip -->
                <div v-if="hoveredIndex === i" class="waterfall-tooltip">
                  <div class="tooltip-title">{{ item.label }}</div>
                  <div class="tooltip-value">{{ item.value > 0 ? '+' : '' }}${{ formatCurrency(item.value) }}</div>
                  <div v-if="item.cumulative" class="tooltip-cumulative">
                    Cumulative: ${{ formatCurrency(item.cumulative) }}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Pie Chart Alternative -->
          <div v-else class="pie-chart-wrapper">
            <div class="pie-chart-container">
              <canvas ref="pieCanvas" width="300" height="300"></canvas>
            </div>
            <div class="pie-legend">
              <div 
                v-for="(item, i) in pieChartData" 
                :key="i"
                class="legend-item"
                :style="{ '--legend-color': item.color }"
              >
                <span class="legend-color"></span>
                <span class="legend-label">{{ item.label }}</span>
                <span class="legend-value">${{ formatCurrency(Math.abs(item.value)) }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Factor & Position Attribution Panels -->
      <div class="attribution-panels">
        <!-- Factor Attribution -->
        <div class="factor-attribution-panel">
          <h3>Factor Attribution</h3>
          <div class="factor-pnl-list">
            <div 
              v-for="factor in factorAttribution" 
              :key="factor.name"
              class="factor-pnl-item"
            >
              <div class="factor-header">
                <span class="factor-name">{{ factor.name }}</span>
                <span class="factor-pnl" :class="factor.pnl > 0 ? 'positive' : 'negative'">
                  {{ factor.pnl > 0 ? '+' : '' }}${{ formatCurrency(factor.pnl) }}
                </span>
              </div>
              <div class="factor-details">
                <div class="factor-contribution">
                  Contribution: {{ (factor.pnl / Math.abs(totalPnl) * 100).toFixed(1) }}%
                </div>
                <div class="factor-positions">
                  {{ factor.activePositions }} positions
                </div>
              </div>
              <div class="factor-bar">
                <div 
                  class="factor-fill"
                  :style="{ 
                    width: Math.abs(factor.pnl) / maxFactorPnl * 100 + '%',
                  }"
                  :class="factor.pnl > 0 ? 'positive' : 'negative'"
                ></div>
              </div>
            </div>
            
            <div class="factor-total">
              <span class="total-label">Total Factor P&L:</span>
              <span class="total-value" :class="totalFactorPnl > 0 ? 'positive' : 'negative'">
                {{ totalFactorPnl > 0 ? '+' : '' }}${{ formatCurrency(totalFactorPnl) }}
              </span>
            </div>
          </div>
        </div>

        <!-- Position Attribution -->
        <div class="position-attribution-panel">
          <h3>Position Attribution</h3>
          <div class="top-contributors">
            <h4>Top Contributors</h4>
            <div class="contributor-list">
              <div 
                v-for="contributor in topContributors" 
                :key="contributor.asset"
                class="contributor-item"
              >
                <div class="contributor-header">
                  <span class="contributor-asset">{{ contributor.asset }}</span>
                  <span class="contributor-type" :class="contributor.direction.toLowerCase()">
                    {{ contributor.direction }}
                  </span>
                </div>
                <div class="contributor-pnl" :class="contributor.pnl > 0 ? 'positive' : 'negative'">
                  {{ contributor.pnl > 0 ? '+' : '' }}${{ formatCurrency(contributor.pnl) }}
                </div>
                <div class="contributor-details">
                  <span class="detail-item">Weight: {{ Math.abs(contributor.weight).toFixed(1) }}%</span>
                  <span class="detail-item">Return: {{ contributor.return.toFixed(1) }}%</span>
                </div>
              </div>
            </div>
            
            <div class="net-contribution">
              <span class="net-label">Net Position Contribution:</span>
              <span class="net-value" :class="netPositionContrib > 0 ? 'positive' : 'negative'">
                {{ netPositionContrib > 0 ? '+' : '' }}${{ formatCurrency(netPositionContrib) }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Risk & Time Analysis -->
      <div class="risk-time-panels">
        <!-- Risk Attribution -->
        <div class="risk-attribution-panel">
          <h3>Risk Attribution</h3>
          <div class="risk-breakdown">
            <div class="risk-item">
              <div class="risk-label">Systematic Risk:</div>
              <div class="risk-percentage">{{ systematicRisk }}%</div>
              <div class="risk-bar">
                <div 
                  class="risk-fill systematic" 
                  :style="{ width: systematicRisk + '%' }"
                ></div>
              </div>
            </div>
            <div class="risk-item">
              <div class="risk-label">Idiosyncratic Risk:</div>
              <div class="risk-percentage">{{ idiosyncraticRisk }}%</div>
              <div class="risk-bar">
                <div 
                  class="risk-fill idiosyncratic" 
                  :style="{ width: idiosyncraticRisk + '%' }"
                ></div>
              </div>
            </div>
            <div class="risk-item">
              <div class="risk-label">Factor Risk:</div>
              <div class="risk-percentage">{{ factorRisk }}%</div>
              <div class="risk-bar">
                <div 
                  class="risk-fill factor" 
                  :style="{ width: factorRisk + '%' }"
                ></div>
              </div>
            </div>
          </div>
          
          <div class="risk-metrics">
            <div class="metric-row">
              <span class="metric-label">Active Risk:</span>
              <span class="metric-value">{{ activeRisk.toFixed(1) }}%</span>
            </div>
            <div class="metric-row">
              <span class="metric-label">Tracking Error:</span>
              <span class="metric-value">{{ trackingError.toFixed(1) }}%</span>
            </div>
          </div>
        </div>

        <!-- Time Analysis -->
        <div class="time-analysis-panel">
          <h3>Time-Based Analysis</h3>
          <div class="hourly-pattern">
            <h4>Hourly P&L Pattern</h4>
            <div class="hourly-chart-container">
              <canvas ref="hourlyCanvas" class="hourly-chart-canvas"></canvas>
              <div class="hourly-axis">
                <div class="axis-labels">
                  <span v-for="hour in [0, 4, 8, 12, 16, 20]" :key="hour" class="axis-label">
                    {{ hour.toString().padStart(2, '0') }}:00
                  </span>
                </div>
              </div>
            </div>
          </div>
          
          <div class="session-analysis">
            <div class="session-item">
              <span class="session-label">NY Open (9:30-16:00):</span>
              <span class="session-pnl positive">+${{ formatCurrency(1890) }}</span>
            </div>
            <div class="session-item">
              <span class="session-label">London Close (16:00-17:00):</span>
              <span class="session-pnl negative">-${{ formatCurrency(234) }}</span>
            </div>
            <div class="session-item">
              <span class="session-label">Asia Session (20:00-04:00):</span>
              <span class="session-pnl positive">+${{ formatCurrency(567) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Advanced Attribution -->
      <div class="advanced-attribution">
        <!-- Rolling Sharpe -->
        <div class="rolling-sharpe-panel">
          <h3>Rolling Sharpe by Factor</h3>
          <div class="sharpe-table">
            <div class="sharpe-header">
              <div class="sharpe-factor">Factor</div>
              <div class="sharpe-period">30d</div>
              <div class="sharpe-period">90d</div>
              <div class="sharpe-period">1y</div>
            </div>
            <div 
              v-for="factor in rollingSharpe" 
              :key="factor.name"
              class="sharpe-row"
            >
              <div class="sharpe-factor">{{ factor.name }}</div>
              <div class="sharpe-value" :class="getSharpeClass(factor.sharpe30d)">
                {{ factor.sharpe30d.toFixed(1) }}
              </div>
              <div class="sharpe-value" :class="getSharpeClass(factor.sharpe90d)">
                {{ factor.sharpe90d.toFixed(1) }}
              </div>
              <div class="sharpe-value" :class="getSharpeClass(factor.sharpe1y)">
                {{ factor.sharpe1y.toFixed(1) }}
              </div>
            </div>
          </div>
        </div>

        <!-- Drawdown Attribution -->
        <div class="drawdown-attribution-panel">
          <h3>Drawdown Attribution</h3>
          <div class="drawdown-analysis">
            <div class="dd-period">
              <span class="dd-label">Max DD Period:</span>
              <span class="dd-dates">{{ maxDDPeriod }}</span>
            </div>
            <div class="dd-breakdown">
              <div class="dd-item">
                <span class="dd-source">Market Beta:</span>
                <span class="dd-contribution negative">-65%</span>
              </div>
              <div class="dd-item">
                <span class="dd-source">Factor Timing:</span>
                <span class="dd-contribution negative">-25%</span>
              </div>
              <div class="dd-item">
                <span class="dd-source">Position Sizing:</span>
                <span class="dd-contribution negative">-10%</span>
              </div>
            </div>
            <div class="dd-total">
              <span class="dd-total-label">Total Explained:</span>
              <span class="dd-total-value">85%</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'

// Reactive state
const selectedPeriod = ref('1d')
const selectedView = ref('waterfall')
const hoveredIndex = ref(-1)
const pieCanvas = ref<HTMLCanvasElement | null>(null)
const hourlyCanvas = ref<HTMLCanvasElement | null>(null)
const totalPnl = ref(14500)

// Computed properties
const totalPnlClass = computed(() => totalPnl.value > 0 ? 'positive' : 'negative')

// Data
const waterfallData = ref([
  { label: 'Starting P&L', value: 10200, cumulative: 10200, type: 'start' },
  { label: 'EWMAC', value: 2300, cumulative: 12500, type: 'factor' },
  { label: 'Momentum', value: 1800, cumulative: 14300, type: 'factor' },
  { label: 'Breakout', value: 900, cumulative: 15200, type: 'factor' },
  { label: 'Trend', value: -300, cumulative: 14900, type: 'factor' },
  { label: 'Transaction Costs', value: -400, cumulative: 14500, type: 'cost' },
  { label: 'Final P&L', value: 14500, cumulative: 14500, type: 'end' }
])

const scaleFactor = computed(() => {
  const maxValue = Math.max(...waterfallData.value.map(item => Math.abs(item.value)))
  return 150 / maxValue // Scale to max 150px height
})

const pieChartData = computed(() => [
  { label: 'EWMAC', value: 2300, color: '#00BF63' },
  { label: 'Momentum', value: 1800, color: '#60a5fa' },
  { label: 'Breakout', value: 900, color: '#22d3ee' },
  { label: 'Trend', value: 300, color: '#FF4757' }, // Absolute value
  { label: 'Transaction Costs', value: 400, color: '#fbbf24' }
])

// Factor attribution
const factorAttribution = ref([
  { name: 'EWMAC', pnl: 2345, activePositions: 8 },
  { name: 'Momentum', pnl: 1789, activePositions: 12 },
  { name: 'Breakout', pnl: 923, activePositions: 6 },
  { name: 'Trend', pnl: -302, activePositions: 4 },
  { name: 'Interaction', pnl: 145, activePositions: 3 }
])

const maxFactorPnl = computed(() => 
  Math.max(...factorAttribution.value.map(f => Math.abs(f.pnl)))
)

const totalFactorPnl = computed(() => 
  factorAttribution.value.reduce((sum, f) => sum + f.pnl, 0)
)

// Position attribution
const topContributors = ref([
  { asset: 'BTC', direction: 'Long', pnl: 1890, weight: 4.2, return: 3.1 },
  { asset: 'ETH', direction: 'Long', pnl: 1234, weight: 3.8, return: 2.8 },
  { asset: 'SOL', direction: 'Long', pnl: 567, weight: 2.1, return: 4.2 },
  { asset: 'DOGE', direction: 'Short', pnl: 234, weight: -2.1, return: -1.8 },
  { asset: 'ADA', direction: 'Long', pnl: -123, weight: 1.8, return: -0.9 }
])

const netPositionContrib = computed(() => 
  topContributors.value.reduce((sum, c) => sum + c.pnl, 0)
)

// Risk data
const systematicRisk = ref(68)
const idiosyncraticRisk = ref(23)
const factorRisk = ref(9)
const activeRisk = ref(2.1)
const trackingError = ref(1.8)

// Hourly P&L data
const hourlyPnL = ref(Array.from({ length: 24 }, (_, i) => ({
  hour: i,
  pnl: (Math.sin(i * 0.3) * 200) + (Math.random() - 0.5) * 100
})))

// Rolling Sharpe ratios
const rollingSharpe = ref([
  { name: 'EWMAC', sharpe30d: 2.1, sharpe90d: 1.8, sharpe1y: 1.6 },
  { name: 'Momentum', sharpe30d: 1.9, sharpe90d: 1.5, sharpe1y: 1.4 },
  { name: 'Breakout', sharpe30d: 1.2, sharpe90d: 1.1, sharpe1y: 0.9 },
  { name: 'Trend', sharpe30d: -0.3, sharpe90d: 0.2, sharpe1y: 0.8 }
])

const maxDDPeriod = ref('Mar 15-28')

// Methods
const formatCurrency = (value: number) => Math.abs(value).toLocaleString()

const getSharpeClass = (sharpe: number) => {
  if (sharpe > 1.5) return 'excellent'
  if (sharpe > 1.0) return 'good'
  if (sharpe > 0) return 'fair'
  return 'poor'
}

// Chart drawing functions
const drawPieChart = () => {
  if (!pieCanvas.value) return
  const ctx = pieCanvas.value.getContext('2d')
  if (!ctx) return

  const centerX = 150
  const centerY = 150
  const radius = 80

  ctx.clearRect(0, 0, 300, 300)

  const total = pieChartData.value.reduce((sum, item) => sum + Math.abs(item.value), 0)
  let currentAngle = -Math.PI / 2

  pieChartData.value.forEach(item => {
    const sliceAngle = (Math.abs(item.value) / total) * 2 * Math.PI
    
    ctx.beginPath()
    ctx.moveTo(centerX, centerY)
    ctx.arc(centerX, centerY, radius, currentAngle, currentAngle + sliceAngle)
    ctx.closePath()
    ctx.fillStyle = item.color
    ctx.fill()
    
    currentAngle += sliceAngle
  })

  // Draw inner circle for donut effect
  ctx.beginPath()
  ctx.arc(centerX, centerY, 30, 0, 2 * Math.PI)
  ctx.fillStyle = '#1f2937'
  ctx.fill()
}

const drawHourlyChart = () => {
  if (!hourlyCanvas.value) return
  const ctx = hourlyCanvas.value.getContext('2d')
  if (!ctx) return

  const canvas = hourlyCanvas.value
  const width = canvas.width
  const height = canvas.height
  const padding = 20

  ctx.clearRect(0, 0, width, height)

  // Draw black background
  ctx.fillStyle = '#000000'
  ctx.fillRect(0, 0, width, height)

  // Calculate scales
  const maxPnl = Math.max(...hourlyPnL.value.map(h => Math.abs(h.pnl)))
  const barWidth = (width - padding * 2) / 24
  const centerY = height / 2

  // Draw zero line (more prominent)
  ctx.strokeStyle = '#6b7280'
  ctx.lineWidth = 2
  ctx.beginPath()
  ctx.moveTo(padding, centerY)
  ctx.lineTo(width - padding, centerY)
  ctx.stroke()

  // Draw grid lines
  ctx.strokeStyle = '#374151'
  ctx.lineWidth = 1
  for (let i = 1; i < 4; i++) {
    const y = (height / 4) * i
    ctx.beginPath()
    ctx.moveTo(padding, y)
    ctx.lineTo(width - padding, y)
    ctx.stroke()
  }

  // Draw bars
  hourlyPnL.value.forEach((hour, i) => {
    const x = padding + i * barWidth
    const barHeight = (Math.abs(hour.pnl) / maxPnl) * (height / 2 - 20)
    const y = hour.pnl > 0 ? centerY - barHeight : centerY
    
    ctx.fillStyle = hour.pnl > 0 ? '#00BF63' : '#FF4757'
    ctx.fillRect(x, y, barWidth * 0.8, Math.abs(barHeight))
    
    // Add subtle border to bars
    ctx.strokeStyle = hour.pnl > 0 ? '#10b981' : '#ef4444'
    ctx.lineWidth = 1
    ctx.strokeRect(x, y, barWidth * 0.8, Math.abs(barHeight))
  })
}

// Lifecycle
onMounted(() => {
  nextTick(() => {
    drawPieChart()
    drawHourlyChart()
  })
})

watch(selectedView, () => {
  if (selectedView.value === 'pie') {
    nextTick(() => drawPieChart())
  }
})
</script>