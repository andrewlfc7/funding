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

    <!-- Multi-Level Attribution Layout -->
    <div class="attribution-layout">
      <!-- Top Panel: P&L Waterfall Chart -->
      <div class="waterfall-panel">
        <h3>P&L Waterfall Chart</h3>
        <div class="waterfall-container">
          <div class="waterfall-chart">
            <div 
              v-for="(item, i) in waterfallData" 
              :key="i"
              class="waterfall-item"
              :class="item.type"
            >
              <div 
                class="waterfall-bar"
                :style="{ 
                  height: Math.abs(item.value) * 2 + 'px',
                  backgroundColor: getWaterfallColor(item.value, item.type)
                }"
              ></div>
              <div class="waterfall-label">{{ item.label }}</div>
              <div class="waterfall-value">
                {{ item.value > 0 ? '+' : '' }}${{ formatCurrency(item.value) }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Middle Panel: Factor & Position Attribution -->
      <div class="attribution-panels">
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
                    backgroundColor: factor.pnl > 0 ? '#00BF63' : '#FF4757'
                  }"
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
                  <span class="contributor-type" :class="contributor.direction">{{ contributor.direction }}</span>
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

      <!-- Risk Attribution & Time Analysis -->
      <div class="risk-time-panels">
        <div class="risk-attribution-panel">
          <h3>Risk Attribution</h3>
          <div class="risk-breakdown">
            <div class="risk-item">
              <div class="risk-label">Systematic Risk:</div>
              <div class="risk-percentage">{{ systematicRisk }}%</div>
              <div class="risk-bar">
                <div class="risk-fill systematic" :style="{ width: systematicRisk + '%' }"></div>
              </div>
            </div>
            <div class="risk-item">
              <div class="risk-label">Idiosyncratic Risk:</div>
              <div class="risk-percentage">{{ idiosyncraticRisk }}%</div>
              <div class="risk-bar">
                <div class="risk-fill idiosyncratic" :style="{ width: idiosyncraticRisk + '%' }"></div>
              </div>
            </div>
            <div class="risk-item">
              <div class="risk-label">Factor Risk:</div>
              <div class="risk-percentage">{{ factorRisk }}%</div>
              <div class="risk-bar">
                <div class="risk-fill factor" :style="{ width: factorRisk + '%' }"></div>
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

        <div class="time-analysis-panel">
          <h3>Time-Based Analysis</h3>
          <div class="hourly-pattern">
            <h4>Hourly P&L Pattern</h4>
            <div class="hourly-chart">
              <div 
                v-for="(hour, i) in hourlyPnL" 
                :key="i"
                class="hour-bar"
                :style="{ 
                  height: Math.abs(hour.pnl) * 5 + 'px',
                  backgroundColor: hour.pnl > 0 ? '#00BF63' : '#FF4757'
                }"
                :title="`${hour.hour}:00 - ${hour.pnl > 0 ? '+' : ''}$${formatCurrency(hour.pnl)}`"
              ></div>
            </div>
            <div class="hour-labels">
              <span v-for="i in 6" :key="i" class="hour-label">
                {{ (i - 1) * 4 }}:00
              </span>
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
import { ref, computed, inject } from 'vue'

// Inject shared state
const signalData = inject('signalData')!

// Local state
const selectedPeriod = ref('1d')
const totalPnl = ref(14500)

const totalPnlClass = computed(() => totalPnl.value > 0 ? 'positive' : 'negative')

// Waterfall data
const waterfallData = ref([
  { label: 'Starting P&L', value: 10200, type: 'start' },
  { label: 'EWMAC', value: 2300, type: 'factor' },
  { label: 'Momentum', value: 1800, type: 'factor' },
  { label: 'Breakout', value: 900, type: 'factor' },
  { label: 'Trend', value: -300, type: 'factor' },
  { label: 'Transaction Costs', value: -400, type: 'cost' },
  { label: 'Final P&L', value: 14500, type: 'end' }
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

// Risk attribution
const systematicRisk = ref(68)
const idiosyncraticRisk = ref(23)
const factorRisk = ref(9)
const activeRisk = ref(2.1)
const trackingError = ref(1.8)

// Time-based analysis
const hourlyPnL = ref(Array.from({ length: 24 }, (_, i) => ({
  hour: i,
  pnl: (Math.random() - 0.5) * 500
})))

// Rolling Sharpe ratios
const rollingSharpe = ref([
  { name: 'EWMAC', sharpe30d: 2.1, sharpe90d: 1.8, sharpe1y: 1.6 },
  { name: 'Momentum', sharpe30d: 1.9, sharpe90d: 1.5, sharpe1y: 1.4 },
  { name: 'Breakout', sharpe30d: 1.2, sharpe90d: 1.1, sharpe1y: 0.9 },
  { name: 'Trend', sharpe30d: -0.3, sharpe90d: 0.2, sharpe1y: 0.8 }
])

const maxDDPeriod = ref('Mar 15-28')

// Utility functions
const formatCurrency = (value: number) => Math.abs(value).toLocaleString()

const getWaterfallColor = (value: number, type: string) => {
  if (type === 'start' || type === 'end') return '#2C3E50'
  if (type === 'cost') return '#FF4757'
  return value > 0 ? '#00BF63' : '#FF4757'
}

const getSharpeClass = (sharpe: number) => {
  if (sharpe > 1.5) return 'excellent'
  if (sharpe > 1.0) return 'good'
  if (sharpe > 0) return 'fair'
  return 'poor'
}
</script>