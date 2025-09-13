<!-- src/components/trend/components/ExposureHeatmap.vue -->
<template>
  <div class="exposure-heatmap-container">
    <div class="heatmap-header">
      <h4>{{ title }}</h4>
      <div class="heatmap-controls">
        <button
          v-for="mode in viewModes"
          :key="mode.value"
          @click="currentMode = mode.value"
          :class="['mode-btn', { active: currentMode === mode.value }]"
        >
          {{ mode.label }}
        </button>
      </div>
    </div>

    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading exposure data...</span>
    </div>

    <div v-else-if="error" class="error-state">
      <span class="error-icon">⚠</span>
      <span>{{ error }}</span>
    </div>

    <div v-else class="heatmap-content">
      <!-- Main heatmap grid -->
      <div class="heatmap-grid" :class="`mode-${currentMode}`">
        <div
          v-for="(position, index) in sortedPositions"
          :key="position.asset"
          class="position-cell"
          :class="{
            'long-position': position.weight > 0,
            'short-position': position.weight < 0,
            'large-position': Math.abs(position.weight) > 0.03
          }"
          :style="getCellStyle(position)"
          @mouseenter="hoveredPosition = position"
          @mouseleave="hoveredPosition = null"
        >
          <!-- Asset label -->
          <div class="cell-header">
            <span class="asset-symbol">{{ position.asset }}</span>
            <span class="position-size">{{ formatWeight(position.weight) }}</span>
          </div>

          <!-- Weight bar -->
          <div class="weight-bar">
            <div 
              class="weight-fill"
              :style="{ 
                width: `${getWeightBarWidth(position.weight)}%`,
                backgroundColor: position.weight > 0 ? '#00BF63' : '#FF4757'
              }"
            ></div>
          </div>

          <!-- Additional metrics based on mode -->
          <div class="cell-metrics">
            <div v-if="currentMode === 'volatility'" class="metric-item">
              <span class="metric-label">Vol:</span>
              <span class="metric-value">{{ formatVol(position.volatility) }}</span>
            </div>
            <div v-else-if="currentMode === 'risk'" class="metric-item">
              <span class="metric-label">Risk:</span>
              <span class="metric-value">{{ formatRisk(position.risk_contribution) }}</span>
            </div>
            <div v-else-if="currentMode === 'pnl'" class="metric-item">
              <span class="metric-label">P&L:</span>
              <span class="metric-value" :class="getPnlClass(position.pnl)">
                {{ formatPnl(position.pnl) }}
              </span>
            </div>
            <div v-else class="metric-item">
              <span class="metric-label">Signal:</span>
              <span class="metric-value">{{ formatSignal(position.signal) }}</span>
            </div>
          </div>

          <!-- Position rank indicator -->
          <div class="rank-indicator" :class="`rank-${getRankCategory(index)}`">
            {{ index + 1 }}
          </div>
        </div>
      </div>

      <!-- Summary statistics -->
      <div class="exposure-summary">
        <div class="summary-row">
          <div class="summary-item long">
            <span class="summary-label">Total Long:</span>
            <span class="summary-value">{{ formatWeight(totalLong) }}</span>
          </div>
          <div class="summary-item short">
            <span class="summary-label">Total Short:</span>
            <span class="summary-value">{{ formatWeight(totalShort) }}</span>
          </div>
          <div class="summary-item net">
            <span class="summary-label">Net:</span>
            <span class="summary-value">{{ formatWeight(netExposure) }}</span>
          </div>
          <div class="summary-item gross">
            <span class="summary-label">Gross:</span>
            <span class="summary-value">{{ formatWeight(grossExposure) }}</span>
          </div>
        </div>

        <!-- Risk-adjusted metrics -->
        <div class="risk-metrics">
          <div class="risk-item">
            <span class="risk-label">Portfolio Vol:</span>
            <span class="risk-value">{{ formatVol(portfolioVol) }}</span>
          </div>
          <div class="risk-item">
            <span class="risk-label">VaR (95%):</span>
            <span class="risk-value">{{ formatVaR(valueAtRisk) }}</span>
          </div>
          <div class="risk-item">
            <span class="risk-label">Concentration:</span>
            <div class="concentration-bar">
              <div 
                class="concentration-fill"
                :style="{ 
                  width: `${concentrationRisk}%`,
                  backgroundColor: getConcentrationColor(concentrationRisk)
                }"
              ></div>
            </div>
            <span class="risk-value">{{ concentrationRisk.toFixed(0) }}%</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Tooltip for hovered position -->
    <div v-if="hoveredPosition" class="position-tooltip" ref="tooltip">
      <div class="tooltip-header">
        <span class="tooltip-asset">{{ hoveredPosition.asset }}</span>
        <span class="tooltip-weight" :class="getWeightClass(hoveredPosition.weight)">
          {{ formatWeight(hoveredPosition.weight) }}
        </span>
      </div>
      <div class="tooltip-metrics">
        <div class="tooltip-metric">
          <span>Signal:</span>
          <span>{{ formatSignal(hoveredPosition.signal) }}</span>
        </div>
        <div class="tooltip-metric">
          <span>Volatility:</span>
          <span>{{ formatVol(hoveredPosition.volatility) }}</span>
        </div>
        <div class="tooltip-metric">
          <span>Expected Return:</span>
          <span>{{ formatReturn(hoveredPosition.expected_return) }}</span>
        </div>
        <div class="tooltip-metric">
          <span>Risk Contribution:</span>
          <span>{{ formatRisk(hoveredPosition.risk_contribution) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue'

interface ExposurePosition {
  asset: string
  weight: number
  volatility: number
  signal: number
  expected_return: number
  risk_contribution: number
  pnl?: number
}

interface Props {
  title?: string
  positions: ExposurePosition[]
  loading?: boolean
  error?: string | null
}

const props = withDefaults(defineProps<Props>(), {
  title: 'Target Exposure Heatmap',
  loading: false,
  error: null
})

// View modes
const viewModes = [
  { value: 'weight', label: 'Weight' },
  { value: 'volatility', label: 'Volatility' },
  { value: 'risk', label: 'Risk' },
  { value: 'pnl', label: 'P&L' }
]

const currentMode = ref('weight')
const hoveredPosition = ref<ExposurePosition | null>(null)
const tooltip = ref<HTMLElement | null>(null)

// Computed properties
const sortedPositions = computed(() => {
  return [...props.positions].sort((a, b) => Math.abs(b.weight) - Math.abs(a.weight))
})

const totalLong = computed(() => {
  return props.positions
    .filter(p => p.weight > 0)
    .reduce((sum, p) => sum + p.weight, 0)
})

const totalShort = computed(() => {
  return props.positions
    .filter(p => p.weight < 0)
    .reduce((sum, p) => sum + p.weight, 0)
})

const netExposure = computed(() => totalLong.value + totalShort.value)
const grossExposure = computed(() => totalLong.value + Math.abs(totalShort.value))

const portfolioVol = computed(() => {
  // Simplified portfolio volatility calculation
  const weightedVols = props.positions.map(p => p.weight * p.volatility)
  return Math.sqrt(weightedVols.reduce((sum, vol) => sum + vol * vol, 0))
})

const valueAtRisk = computed(() => {
  // 95% VaR approximation
  return portfolioVol.value * 1.645 * Math.sqrt(252 / 365)
})

const concentrationRisk = computed(() => {
  const weights = props.positions.map(p => Math.abs(p.weight))
  const maxWeight = Math.max(...weights)
  return (maxWeight / 0.05) * 100 // Normalize to 5% max position
})

// Styling functions
function getCellStyle(position: ExposurePosition) {
  const absWeight = Math.abs(position.weight)
  const opacity = Math.min(0.3 + absWeight * 10, 0.9)
  
  let backgroundColor = position.weight > 0 ? 'rgba(0, 191, 99, ' : 'rgba(255, 71, 87, '
  
  if (currentMode.value === 'volatility') {
    const volIntensity = Math.min(position.volatility, 2) / 2
    backgroundColor = `rgba(255, 165, 2, ${volIntensity})`
  } else if (currentMode.value === 'risk') {
    const riskIntensity = Math.min(position.risk_contribution, 0.1) / 0.1
    backgroundColor = `rgba(255, 107, 107, ${riskIntensity})`
  }
  
  return {
    backgroundColor: backgroundColor + opacity + ')',
    transform: absWeight > 0.03 ? 'scale(1.05)' : 'scale(1)',
    zIndex: absWeight > 0.03 ? 10 : 1
  }
}

function getWeightBarWidth(weight: number): number {
  return Math.min(Math.abs(weight) * 2000, 100) // Scale for visibility
}

function getRankCategory(index: number): string {
  if (index < 3) return 'top'
  if (index < 8) return 'mid'
  return 'low'
}

function getWeightClass(weight: number): string {
  return weight > 0 ? 'positive' : 'negative'
}

function getPnlClass(pnl: number | undefined): string {
  if (!pnl) return 'neutral'
  return pnl > 0 ? 'positive' : 'negative'
}

function getConcentrationColor(concentration: number): string {
  if (concentration < 60) return '#00BF63'
  if (concentration < 80) return '#FFA502'
  return '#FF4757'
}

// Formatting functions
function formatWeight(weight: number): string {
  return `${(weight * 100).toFixed(1)}%`
}

function formatVol(vol: number): string {
  return `${(vol * 100).toFixed(0)}%`
}

function formatSignal(signal: number): string {
  return signal.toFixed(2)
}

function formatReturn(ret: number): string {
  return `${(ret * 100).toFixed(1)}%`
}

function formatRisk(risk: number): string {
  return `${(risk * 100).toFixed(1)}%`
}

function formatPnl(pnl: number | undefined): string {
  if (!pnl) return 'N/A'
  return `${pnl > 0 ? '+' : ''}${(pnl * 100).toFixed(1)}%`
}

function formatVaR(var95: number): string {
  return `${(var95 * 100).toFixed(1)}%`
}

// Tooltip positioning
watch(hoveredPosition, async () => {
  if (hoveredPosition.value) {
    await nextTick()
    // Position tooltip logic would go here
  }
})
</script>
