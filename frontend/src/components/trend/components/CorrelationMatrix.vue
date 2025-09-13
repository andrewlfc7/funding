<template>
  <div class="correlation-matrix-container">
    <div class="matrix-header">
      <h4>{{ title }}</h4>
      <div class="matrix-controls">
        <select v-model="selectedPeriod" class="period-select">
          <option value="30d">30 Days</option>
          <option value="60d">60 Days</option>
          <option value="90d">90 Days</option>
        </select>
      </div>
    </div>
    
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading correlations...</span>
    </div>
    
    <div v-else-if="error" class="error-state">
      <span class="error-icon">⚠</span>
      <span>{{ error }}</span>
    </div>
    
    <div v-else class="matrix-content">
      <!-- Correlation Heatmap -->
      <div class="correlation-heatmap">
        <div class="matrix-labels">
          <div class="label-spacer"></div>
          <div 
            v-for="asset in assets" 
            :key="asset"
            class="column-label"
          >
            {{ asset }}
          </div>
        </div>
        
        <div 
          v-for="(row, rowIndex) in correlationMatrix" 
          :key="rowIndex"
          class="matrix-row"
        >
          <div class="row-label">{{ assets[rowIndex] }}</div>
          <div 
            v-for="(correlation, colIndex) in row" 
            :key="colIndex"
            class="correlation-cell"
            :class="getCorrClass(correlation)"
            :style="{ 
              backgroundColor: getCorrColor(correlation),
              color: getCorrTextColor(correlation)
            }"
            :title="`${assets[rowIndex]} vs ${assets[colIndex]}: ${correlation.toFixed(3)}`"
            @click="selectCell(rowIndex, colIndex)"
          >
            {{ formatCorrelation(correlation) }}
          </div>
        </div>
      </div>
      
      <!-- Correlation Statistics -->
      <div class="correlation-stats">
        <div class="stat-group">
          <h5>Matrix Statistics</h5>
          <div class="stats-grid">
            <div class="stat-item">
              <span class="stat-label">Avg Correlation:</span>
              <span class="stat-value">{{ formatCorrelation(avgCorrelation) }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Max Correlation:</span>
              <span class="stat-value positive">{{ formatCorrelation(maxCorrelation) }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Min Correlation:</span>
              <span class="stat-value negative">{{ formatCorrelation(minCorrelation) }}</span>
            </div>
            <div class="stat-item">
              <span class="stat-label">Condition Number:</span>
              <span class="stat-value" :class="getConditionClass(conditionNumber)">
                {{ conditionNumber.toFixed(2) }}
              </span>
            </div>
          </div>
        </div>
        
        <!-- Selected Cell Info -->
        <div v-if="selectedCell" class="selected-cell-info">
          <h5>Pair Analysis</h5>
          <div class="pair-details">
            <div class="pair-header">
              <span class="asset-pair">{{ selectedPair.asset1 }} × {{ selectedPair.asset2 }}</span>
              <span class="pair-correlation" :class="getCorrClass(selectedPair.correlation)">
                {{ formatCorrelation(selectedPair.correlation) }}
              </span>
            </div>
            <div class="pair-metrics">
              <div class="pair-metric">
                <span>Rolling 30d:</span>
                <span>{{ formatCorrelation(selectedPair.rolling30d) }}</span>
              </div>
              <div class="pair-metric">
                <span>Volatility:</span>
                <span>{{ (selectedPair.volatility * 100).toFixed(1) }}%</span>
              </div>
              <div class="pair-metric">
                <span>Relationship:</span>
                <span :class="getRelationshipClass(selectedPair.correlation)">
                  {{ getRelationshipLabel(selectedPair.correlation) }}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      <!-- Color Scale Legend -->
      <div class="color-scale">
        <span class="scale-label">Correlation Scale:</span>
        <div class="scale-gradient">
          <div class="scale-markers">
            <span>-1.0</span>
            <span>-0.5</span>
            <span>0.0</span>
            <span>0.5</span>
            <span>1.0</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'

interface Props {
  title?: string
  assets: string[]
  correlationData: number[][]
  loading?: boolean
  error?: string | null
}

const props = withDefaults(defineProps<Props>(), {
  title: 'Asset Correlation Matrix',
  loading: false,
  error: null
})

const selectedPeriod = ref('90d')
const selectedCell = ref<{ row: number; col: number } | null>(null)

// Computed correlation matrix
const correlationMatrix = computed(() => props.correlationData)

// Matrix statistics
const avgCorrelation = computed(() => {
  const matrix = correlationMatrix.value
  if (!matrix.length) return 0
  
  let sum = 0
  let count = 0
  
  for (let i = 0; i < matrix.length; i++) {
    for (let j = i + 1; j < matrix[i].length; j++) {
      sum += matrix[i][j]
      count++
    }
  }
  
  return count > 0 ? sum / count : 0
})

const maxCorrelation = computed(() => {
  const matrix = correlationMatrix.value
  if (!matrix.length) return 0
  
  let max = -1
  for (let i = 0; i < matrix.length; i++) {
    for (let j = i + 1; j < matrix[i].length; j++) {
      max = Math.max(max, matrix[i][j])
    }
  }
  return max
})

const minCorrelation = computed(() => {
  const matrix = correlationMatrix.value
  if (!matrix.length) return 0
  
  let min = 1
  for (let i = 0; i < matrix.length; i++) {
    for (let j = i + 1; j < matrix[i].length; j++) {
      min = Math.min(min, matrix[i][j])
    }
  }
  return min
})

const conditionNumber = computed(() => {
  // Simplified condition number approximation
  const matrix = correlationMatrix.value
  if (!matrix.length) return 1
  
  const eigenvalues = matrix.map(row => 
    row.reduce((sum, val) => sum + Math.abs(val), 0)
  )
  
  const maxEig = Math.max(...eigenvalues)
  const minEig = Math.min(...eigenvalues.filter(val => val > 0.001))
  
  return maxEig / minEig
})

// Selected cell details
const selectedPair = computed(() => {
  if (!selectedCell.value) return null
  
  const { row, col } = selectedCell.value
  const correlation = correlationMatrix.value[row][col]
  
  return {
    asset1: props.assets[row],
    asset2: props.assets[col],
    correlation,
    rolling30d: correlation * (0.9 + Math.random() * 0.2), // Mock rolling correlation
    volatility: Math.random() * 0.1 + 0.05 // Mock volatility
  }
})

// Methods
function selectCell(row: number, col: number) {
  if (row === col) return // Don't select diagonal
  selectedCell.value = { row, col }
}

function formatCorrelation(corr: number): string {
  return corr.toFixed(3)
}

function getCorrColor(correlation: number): string {
  const abs = Math.abs(correlation)
  
  if (correlation > 0.5) {
    const intensity = (correlation - 0.5) / 0.5
    return `rgba(16, 185, 129, ${0.2 + intensity * 0.6})`
  } else if (correlation < -0.5) {
    const intensity = (Math.abs(correlation) - 0.5) / 0.5
    return `rgba(239, 68, 68, ${0.2 + intensity * 0.6})`
  } else {
    const intensity = abs / 0.5
    return `rgba(245, 158, 11, ${0.1 + intensity * 0.3})`
  }
}

function getCorrTextColor(correlation: number): string {
  const abs = Math.abs(correlation)
  if (abs > 0.7) return '#ffffff'
  return '#e5e7eb'
}

function getCorrClass(correlation: number): string {
  if (correlation > 0.7) return 'high-positive'
  if (correlation > 0.3) return 'moderate-positive'
  if (correlation < -0.7) return 'high-negative'
  if (correlation < -0.3) return 'moderate-negative'
  return 'low-correlation'
}

function getConditionClass(conditionNum: number): string {
  if (conditionNum > 20) return 'high-condition'
  if (conditionNum > 10) return 'moderate-condition'
  return 'low-condition'
}

function getRelationshipClass(correlation: number): string {
  if (Math.abs(correlation) > 0.7) return 'strong-relationship'
  if (Math.abs(correlation) > 0.3) return 'moderate-relationship'
  return 'weak-relationship'
}

function getRelationshipLabel(correlation: number): string {
  const abs = Math.abs(correlation)
  const direction = correlation > 0 ? 'Positive' : 'Negative'
  
  if (abs > 0.7) return `Strong ${direction}`
  if (abs > 0.3) return `Moderate ${direction}`
  return 'Weak'
}

onMounted(() => {
  // Auto-select the first non-diagonal cell
  if (props.assets.length > 1) {
    selectCell(0, 1)
  }
})
</script>
