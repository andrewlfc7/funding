<template>
  <div class="heatmap-chart-wrapper">
    <div class="heatmap-container">
      <!-- Top labels -->
      <div class="heatmap-labels-top">
        <div class="corner-spacer"></div>
        <div class="x-labels">
          <div 
            v-for="(label, i) in labels" 
            :key="`x-${i}`" 
            class="x-label"
            :title="label"
          >
            <span>{{ label }}</span>
          </div>
        </div>
      </div>
      
      <!-- Main content row -->
      <div class="heatmap-main">
        <!-- Left labels -->
        <div class="y-labels">
          <div 
            v-for="(label, i) in labels" 
            :key="`y-${i}`" 
            class="y-label"
            :title="label"
          >
            <span>{{ label }}</span>
          </div>
        </div>
        
        <!-- Heatmap canvas -->
        <div class="heatmap-chart-container" ref="containerRef">
          <canvas ref="chartCanvas" @mousemove="handleMouseMove" @mouseleave="handleMouseLeave"></canvas>
          <!-- Tooltip -->
          <div 
            v-if="tooltip.show" 
            class="heatmap-tooltip"
            :style="{
              left: tooltip.x + 'px',
              top: tooltip.y + 'px'
            }"
          >
            <div class="tooltip-header">{{ tooltip.rowLabel }} / {{ tooltip.colLabel }}</div>
            <div class="tooltip-value">{{ formatValue(tooltip.value) }}</div>
          </div>
        </div>
      </div>
    </div>

    <div class="heatmap-legend" v-if="showLegend">
      <span class="legend-min">{{ formatLegendValue(min) }}</span>
      <div class="legend-gradient" :class="colorScheme" ref="legendRef"></div>
      <span class="legend-max">{{ formatLegendValue(max) }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted, nextTick, computed } from 'vue'

type Matrix = number[][]
type HeatmapInput =
  | { labels: string[]; data: Matrix }
  | { coins: string[]; matrix: Matrix }
  | number[][] 


  

interface Props {
  data: HeatmapInput
  min?: number
  max?: number
  colorScheme?: 'correlation' | 'beta' | 'covariance' | 'default'
  showLegend?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  min: -1,
  max: 1,
  colorScheme: 'correlation',
  showLegend: true
})

const chartCanvas = ref<HTMLCanvasElement>()
const containerRef = ref<HTMLDivElement>()
const legendRef = ref<HTMLDivElement>()
let animationId: number | null = null
let resizeObserver: ResizeObserver | null = null

// Tooltip state
const tooltip = ref({
  show: false,
  x: 0,
  y: 0,
  rowLabel: '',
  colLabel: '',
  value: 0
})

// Unify inputs
const labels = computed<string[]>(() => {
  const d = props.data as any
  return (d.labels || d.coins || []) as string[]
})

const matrix = computed<Matrix>(() => {
  const d = props.data as any
  return (d.data || d.matrix || []) as Matrix
})

// Color functions
function getColorForValue(value: number): string {
  const normalizedValue = Math.max(0, Math.min(1, (value - props.min) / (props.max - props.min)))
  
  if (props.colorScheme === 'correlation') {
    // Improved correlation color scheme
    const clampedValue = Math.max(-1, Math.min(1, value))
    const t = (clampedValue + 1) / 2
    
    if (t < 0.5) {
      // Red to yellow
      const localT = t * 2
      const r = 220
      const g = Math.round(60 + (220 - 60) * localT)
      const b = 60
      return `rgb(${r}, ${g}, ${b})`
    } else {
      // Yellow to green
      const localT = (t - 0.5) * 2
      const r = Math.round(220 - (220 - 34) * localT)
      const g = Math.round(220 - (220 - 197) * localT)
      const b = Math.round(60 + (120 - 60) * localT)
      return `rgb(${r}, ${g}, ${b})`
    }
  } else if (props.colorScheme === 'beta') {
    // Blue gradient for beta
    const t = normalizedValue
    const r = Math.round(30 + (59 - 30) * t)
    const g = Math.round(30 + (130 - 30) * t)
    const b = Math.round(100 + (246 - 100) * t)
    return `rgb(${r}, ${g}, ${b})`
  } else if (props.colorScheme === 'covariance') {
    // Purple to orange gradient for covariance
    const t = normalizedValue
    if (t < 0.5) {
      const localT = t * 2
      const r = Math.round(67 + (147 - 67) * localT)
      const g = Math.round(56 + (51 - 56) * localT)
      const b = Math.round(137 + (234 - 137) * localT)
      return `rgb(${r}, ${g}, ${b})`
    } else {
      const localT = (t - 0.5) * 2
      const r = Math.round(147 + (251 - 147) * localT)
      const g = Math.round(51 + (146 - 51) * localT)
      const b = Math.round(234 - (234 - 38) * localT)
      return `rgb(${r}, ${g}, ${b})`
    }
  } else {
    // Default purple gradient
    const t = normalizedValue
    const r = Math.round(67 + (147 - 67) * t)
    const g = Math.round(56 + (51 - 56) * t)
    const b = Math.round(137 + (234 - 137) * t)
    return `rgb(${r}, ${g}, ${b})`
  }
}

// Format value for display
function formatValue(value: number): string {
  if (props.colorScheme === 'covariance') {
    // Handle very small covariance values
    if (Math.abs(value) < 0.0001) {
      return value.toExponential(2)
    }
    return value.toFixed(6)
  }
  return value.toFixed(3)
}

function formatLegendValue(value: number): string {
  if (props.colorScheme === 'covariance' && Math.abs(value) < 0.001) {
    return value.toExponential(1)
  }
  return value.toFixed(2)
}

// Mouse handlers
function handleMouseMove(event: MouseEvent) {
  const canvas = chartCanvas.value
  const container = containerRef.value
  if (!canvas || !container || !matrix.value.length) return

  const rect = canvas.getBoundingClientRect()
  const x = event.clientX - rect.left
  const y = event.clientY - rect.top

  const numRows = matrix.value.length
  const numCols = matrix.value[0]?.length || 0
  const cellWidth = rect.width / numCols
  const cellHeight = rect.height / numRows

  const col = Math.floor(x / cellWidth)
  const row = Math.floor(y / cellHeight)

  if (row >= 0 && row < numRows && col >= 0 && col < numCols) {
    tooltip.value = {
      show: true,
      x: Math.min(x + 10, rect.width - 150),
      y: Math.max(y - 40, 10),
      rowLabel: labels.value[row] || `Row ${row}`,
      colLabel: labels.value[col] || `Col ${col}`,
      value: matrix.value[row][col]
    }
  }
}

function handleMouseLeave() {
  tooltip.value.show = false
}

function drawHeatmap() {
  const canvas = chartCanvas.value
  const container = containerRef.value
  if (!canvas || !container || !matrix.value.length) return

  const ctx = canvas.getContext('2d')
  if (!ctx) return

  // Get container dimensions
  const rect = container.getBoundingClientRect()
  const width = rect.width
  const height = rect.height

  // Set canvas size with device pixel ratio for sharp rendering
  const dpr = window.devicePixelRatio || 1
  canvas.width = width * dpr
  canvas.height = height * dpr
  canvas.style.width = `${width}px`
  canvas.style.height = `${height}px`
  ctx.scale(dpr, dpr)

  // Clear canvas
  ctx.clearRect(0, 0, width, height)

  // Calculate dimensions
  const numRows = matrix.value.length
  const numCols = matrix.value[0]?.length || 0
  const cellWidth = width / numCols
  const cellHeight = height / numRows

  // Draw cells
  for (let i = 0; i < numRows; i++) {
    for (let j = 0; j < numCols; j++) {
      const value = matrix.value[i][j]
      const x = j * cellWidth
      const y = i * cellHeight

      // Draw cell
      ctx.fillStyle = getColorForValue(value)
      ctx.fillRect(x, y, cellWidth, cellHeight)

      // Draw subtle grid
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)'
      ctx.lineWidth = 0.5
      ctx.strokeRect(x, y, cellWidth, cellHeight)
    }
  }
}

// Draw gradient legend
function drawLegendGradient() {
  if (!legendRef.value || !props.showLegend) return

  const canvas = document.createElement('canvas')
  const ctx = canvas.getContext('2d')
  if (!ctx) return

  canvas.width = 200
  canvas.height = 16
  
  const gradient = ctx.createLinearGradient(0, 0, 200, 0)
  
  // Add color stops based on color scheme
  for (let i = 0; i <= 10; i++) {
    const t = i / 10
    const value = props.min + (props.max - props.min) * t
    gradient.addColorStop(t, getColorForValue(value))
  }
  
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, 200, 16)
  
  legendRef.value.style.background = `url(${canvas.toDataURL()})`
  legendRef.value.style.backgroundSize = 'cover'
}

function scheduleDraw() {
  if (animationId) cancelAnimationFrame(animationId)
  animationId = requestAnimationFrame(() => {
    drawHeatmap()
    drawLegendGradient()
    animationId = null
  })
}

// Setup resize observer
function setupResizeObserver() {
  if (!containerRef.value) return
  
  resizeObserver = new ResizeObserver(() => {
    scheduleDraw()
  })
  
  resizeObserver.observe(containerRef.value)
}

onMounted(async () => {
  await nextTick()
  setupResizeObserver()
  scheduleDraw()
})

watch(
  () => [props.data, props.colorScheme, props.min, props.max],
  () => {
    scheduleDraw()
  },
  { deep: true }
)

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect()
  }
  if (animationId) {
    cancelAnimationFrame(animationId)
  }
})
</script>
