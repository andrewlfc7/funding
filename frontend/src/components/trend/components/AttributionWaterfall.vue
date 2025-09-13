<template>
  <div class="attribution-waterfall-container">
    <div class="waterfall-header">
      <h4>{{ title }}</h4>
      <div class="waterfall-controls">
        <select v-model="selectedPeriod" class="period-select">
          <option value="1d">1 Day</option>
          <option value="7d">7 Days</option>
          <option value="30d">30 Days</option>
          <option value="90d">90 Days</option>
        </select>
      </div>
    </div>
    
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading attribution...</span>
    </div>
    
    <div v-else-if="error" class="error-state">
      <span class="error-icon">⚠</span>
      <span>{{ error }}</span>
    </div>
    
    <div v-else class="waterfall-content">
      <!-- Main Waterfall Chart -->
      <div class="waterfall-chart">
        <canvas ref="chartCanvas"></canvas>
      </div>
      
      <!-- Attribution Breakdown -->
      <div class="attribution-breakdown">
        <div class="breakdown-section">
          <h5>Factor Attribution</h5>
          <div class="factor-items">
            <div 
              v-for="factor in factorAttribution" 
              :key="factor.name"
              class="attribution-item"
              :class="getAttributionClass(factor.value)"
            >
              <div class="item-header">
                <span class="item-name">{{ factor.name }}</span>
                <span class="item-value" :class="getValueClass(factor.value)">
                  {{ formatAttribution(factor.value) }}
                </span>
              </div>
              <div class="item-bar">
                <div 
                  class="item-fill"
                  :style="{ 
                    width: `${Math.abs(factor.value) / maxAbsValue * 100}%`,
                    backgroundColor: getBarColor(factor.value)
                  }"
                ></div>
              </div>
              <div class="item-details">
                <span class="contribution">{{ (factor.value / totalPnl * 100).toFixed(1) }}% of total</span>
                <span class="significance" :class="getSignificanceClass(factor.tStat)">
                  t-stat: {{ factor.tStat.toFixed(2) }}
                </span>
              </div>
            </div>
          </div>
        </div>
        
        <div class="breakdown-section">
          <h5>Asset Attribution</h5>
          <div class="asset-items">
            <div 
              v-for="asset in topAssetAttribution" 
              :key="asset.name"
              class="attribution-item"
              :class="getAttributionClass(asset.value)"
            >
              <div class="item-header">
                <span class="item-name">{{ asset.name }}</span>
                <span class="item-value" :class="getValueClass(asset.value)">
                  {{ formatAttribution(asset.value) }}
                </span>
              </div>
              <div class="item-bar">
                <div 
                  class="item-fill"
                  :style="{ 
                    width: `${Math.abs(asset.value) / maxAbsValue * 100}%`,
                    backgroundColor: getBarColor(asset.value)
                  }"
                ></div>
              </div>
              <div class="item-details">
                <span class="position-size">{{ asset.weight.toFixed(1) }}% position</span>
                <span class="asset-return" :class="getValueClass(asset.return)">
                  {{ (asset.return * 100).toFixed(1) }}% return
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- Summary Statistics -->
      <div class="attribution-summary">
        <div class="summary-metrics">
          <div class="metric-card">
            <div class="metric-label">Total P&L</div>
            <div class="metric-value total-pnl" :class="getValueClass(totalPnl)">
              {{ formatAttribution(totalPnl) }}
            </div>
            <div class="metric-change">
              {{ (totalPnl / startingValue * 100).toFixed(2) }}% return
            </div>
          </div>
          
          <div class="metric-card">
            <div class="metric-label">Factor Contribution</div>
            <div class="metric-value" :class="getValueClass(factorContribution)">
              {{ formatAttribution(factorContribution) }}
            </div>
            <div class="metric-change">
              {{ (factorContribution / totalPnl * 100).toFixed(1) }}% of total
            </div>
          </div>
          
          <div class="metric-card">
            <div class="metric-label">Selection Effect</div>
            <div class="metric-value" :class="getValueClass(selectionEffect)">
              {{ formatAttribution(selectionEffect) }}
            </div>
            <div class="metric-change">
              {{ (selectionEffect / totalPnl * 100).toFixed(1) }}% of total
            </div>
          </div>
          
          <div class="metric-card">
            <div class="metric-label">Transaction Costs</div>
            <div class="metric-value negative">
              {{ formatAttribution(transactionCosts) }}
            </div>
            <div class="metric-change">
              {{ (Math.abs(transactionCosts) / totalPnl * 100).toFixed(1) }}% drag
            </div>
          </div>
        </div>
        
        <div class="risk-metrics">
          <h5>Risk-Adjusted Performance</h5>
          <div class="risk-grid">
            <div class="risk-item">
              <span class="risk-label">Sharpe Ratio:</span>
              <span class="risk-value" :class="getSharpeClass(sharpeRatio)">{{ sharpeRatio.toFixed(2) }}</span>
            </div>
            <div class="risk-item">
              <span class="risk-label">Information Ratio:</span>
              <span class="risk-value" :class="getIrClass(informationRatio)">{{ informationRatio.toFixed(2) }}</span>
            </div>
            <div class="risk-item">
              <span class="risk-label">Max Drawdown:</span>
              <span class="risk-value negative">{{ (maxDrawdown * 100).toFixed(1) }}%</span>
            </div>
            <div class="risk-item">
              <span class="risk-label">Hit Rate:</span>
              <span class="risk-value" :class="getHitRateClass(hitRate)">{{ (hitRate * 100).toFixed(1) }}%</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { Chart, registerables, type ChartConfiguration } from 'chart.js'

Chart.register(...registerables)

interface AttributionFactor {
  name: string
  value: number
  tStat: number
}

interface AttributionAsset {
  name: string
  value: number
  weight: number
  return: number
}

interface Props {
  title?: string
  factorData: AttributionFactor[]
  assetData: AttributionAsset[]
  startingValue: number
  endingValue: number
  loading?: boolean
  error?: string | null
}

const props = withDefaults(defineProps<Props>(), {
  title: 'Performance Attribution Waterfall',
  loading: false,
  error: null
})

const selectedPeriod = ref('30d')
const chartCanvas = ref<HTMLCanvasElement | null>(null)
let chart: Chart | null = null

// Computed values
const totalPnl = computed(() => props.endingValue - props.startingValue)

const factorAttribution = computed(() => 
  [...props.factorData].sort((a, b) => Math.abs(b.value) - Math.abs(a.value))
)

const topAssetAttribution = computed(() => 
  [...props.assetData]
    .sort((a, b) => Math.abs(b.value) - Math.abs(a.value))
    .slice(0, 8)
)

const maxAbsValue = computed(() => {
  const allValues = [
    ...factorAttribution.value.map(f => Math.abs(f.value)),
    ...topAssetAttribution.value.map(a => Math.abs(a.value))
  ]
  return Math.max(...allValues, 1)
})

const factorContribution = computed(() => 
  factorAttribution.value.reduce((sum, f) => sum + f.value, 0)
)

const selectionEffect = computed(() => 
  topAssetAttribution.value.reduce((sum, a) => sum + a.value, 0)
)

const transactionCosts = computed(() => totalPnl.value * -0.015) // Mock -1.5% transaction costs

// Performance metrics (mock data - would come from API)
const sharpeRatio = computed(() => 1.24 + Math.random() * 0.5)
const informationRatio = computed(() => 0.85 + Math.random() * 0.4)
const maxDrawdown = computed(() => -(0.08 + Math.random() * 0.05))
const hitRate = computed(() => 0.58 + Math.random() * 0.15)

// Chart data for waterfall
const waterfallData = computed(() => {
  const data = []
  let runningTotal = props.startingValue
  
  // Starting value
  data.push({
    label: 'Starting Value',
    value: props.startingValue,
    cumulative: props.startingValue,
    type: 'start'
  })
  
  // Factor contributions
  factorAttribution.value.forEach(factor => {
    runningTotal += factor.value
    data.push({
      label: factor.name,
      value: factor.value,
      cumulative: runningTotal,
      type: factor.value > 0 ? 'positive' : 'negative'
    })
  })
  
  // Transaction costs
  runningTotal += transactionCosts.value
  data.push({
    label: 'Transaction Costs',
    value: transactionCosts.value,
    cumulative: runningTotal,
    type: 'negative'
  })
  
  // Ending value
  data.push({
    label: 'Ending Value',
    value: props.endingValue,
    cumulative: props.endingValue,
    type: 'end'
  })
  
  return data
})

// Methods
function formatAttribution(value: number): string {
  return `${value.toLocaleString('en-US', { 
    minimumFractionDigits: 0, 
    maximumFractionDigits: 0 
  })}`
}

function getAttributionClass(value: number): string {
  return value > 0 ? 'positive-contribution' : 'negative-contribution'
}

function getValueClass(value: number): string {
  if (value > 0) return 'positive'
  if (value < 0) return 'negative'
  return 'neutral'
}

function getBarColor(value: number): string {
  return value > 0 ? 'var(--color-positive)' : 'var(--color-negative)'
}

function getSignificanceClass(tStat: number): string {
  const abs = Math.abs(tStat)
  if (abs > 2.58) return 'highly-significant' // 1% level
  if (abs > 1.96) return 'significant' // 5% level
  return 'not-significant'
}

function getSharpeClass(sharpe: number): string {
  if (sharpe > 1.5) return 'excellent'
  if (sharpe > 1.0) return 'good'
  if (sharpe > 0.5) return 'fair'
  return 'poor'
}

function getIrClass(ir: number): string {
  if (ir > 0.75) return 'excellent'
  if (ir > 0.5) return 'good'
  if (ir > 0.25) return 'fair'
  return 'poor'
}

function getHitRateClass(hitRate: number): string {
  if (hitRate > 0.6) return 'excellent'
  if (hitRate > 0.55) return 'good'
  if (hitRate > 0.5) return 'fair'
  return 'poor'
}

function createWaterfallChart() {
  if (!chartCanvas.value || chart) return
  const ctx = chartCanvas.value.getContext('2d')
  if (!ctx) return
  
  const labels = waterfallData.value.map(d => d.label)
  const values = waterfallData.value.map(d => d.value)
  const cumulativeValues = waterfallData.value.map(d => d.cumulative)
  
  const config: ChartConfiguration = {
    type: 'bar',
    data: {
      labels,
      datasets: [{
        label: 'Attribution',
        data: values,
        backgroundColor: values.map((v, i) => {
          const item = waterfallData.value[i]
          if (item.type === 'start' || item.type === 'end') return 'var(--color-info)'
          return v > 0 ? 'var(--color-positive)' : 'var(--color-negative)'
        }),
        borderColor: 'var(--border-primary)',
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
            label: (context) => {
              const item = waterfallData.value[context.dataIndex]
              return [
                `${item.label}: ${formatAttribution(item.value)}`,
                `Cumulative: ${formatAttribution(item.cumulative)}`
              ]
            }
          }
        }
      },
      scales: {
        x: {
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: { 
            color: '#ECF0F1',
            maxRotation: 45
          }
        },
        y: {
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: { 
            color: '#ECF0F1',
            callback: (value) => formatAttribution(Number(value))
          }
        }
      }
    }
  }
  
  chart = new Chart(ctx, config)
}

function updateChart() {
  if (!chart) {
    createWaterfallChart()
    return
  }
  
  const labels = waterfallData.value.map(d => d.label)
  const values = waterfallData.value.map(d => d.value)
  
  chart.data.labels = labels
  chart.data.datasets[0].data = values
  chart.data.datasets[0].backgroundColor = values.map((v, i) => {
    const item = waterfallData.value[i]
    if (item.type === 'start' || item.type === 'end') return 'var(--color-info)'
    return v > 0 ? 'var(--color-positive)' : 'var(--color-negative)'
  })
  chart.update('none')
}

onMounted(createWaterfallChart)
onUnmounted(() => {
  chart?.destroy()
  chart = null
})

watch(() => [props.factorData, props.assetData], updateChart, { deep: true })
</script>

