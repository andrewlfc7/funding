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
          >
            {{ label }}
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
          >
            {{ label }}
          </div>
        </div>
        
        <!-- Heatmap canvas -->
        <div class="heatmap-chart-container" ref="containerRef">
          <canvas ref="chartCanvas"></canvas>
        </div>
      </div>
    </div>

    <div class="heatmap-legend" v-if="showLegend">
      <span class="legend-min">{{ min.toFixed(2) }}</span>
      <div class="legend-gradient" :class="colorScheme"></div>
      <span class="legend-max">{{ max.toFixed(2) }}</span>
    </div>
  </div>
</template>


<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted, nextTick, computed } from 'vue'

type Matrix = number[][]
type HeatmapInput =
  | { labels: string[]; data: Matrix }
  | { coins: string[]; matrix: Matrix }

interface Props {
  data: HeatmapInput
  min?: number
  max?: number
  colorScheme?: 'correlation' | 'beta' | 'default'
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
let animationId: number | null = null
let resizeObserver: ResizeObserver | null = null

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
  const normalizedValue = (value - props.min) / (props.max - props.min)
  
  if (props.colorScheme === 'correlation') {
    // Red to yellow to green for correlation
    const clampedValue = Math.max(-1, Math.min(1, value))
    const t = (clampedValue + 1) / 2 // Normalize to 0-1
    
    if (t < 0.5) {
      // Red to yellow
      const localT = t * 2
      const r = 239
      const g = Math.round(68 + (204 - 68) * localT)
      const b = 68
      return `rgb(${r}, ${g}, ${b})`
    } else {
      // Yellow to green
      const localT = (t - 0.5) * 2
      const r = Math.round(250 - (250 - 34) * localT)
      const g = Math.round(204 - (204 - 197) * localT)
      const b = Math.round(21 + (94 - 21) * localT)
      return `rgb(${r}, ${g}, ${b})`
    }
  } else if (props.colorScheme === 'beta') {
    // Blue gradient for beta
    const opacity = 0.2 + normalizedValue * 0.6
    return `rgba(59, 130, 246, ${opacity})`
  } else {
    // Default purple gradient
    const opacity = 0.2 + normalizedValue * 0.8
    return `rgba(99, 102, 241, ${opacity})`
  }
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

  // Calculate dimensions - no margins needed since labels are outside
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
      ctx.fillRect(x, y, cellWidth - 1, cellHeight - 1)

      // Draw cell border
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)'
      ctx.lineWidth = 0.5
      ctx.strokeRect(x, y, cellWidth - 1, cellHeight - 1)

      // Draw value text for small matrices
      if (numRows <= 10 && numCols <= 10 && cellWidth > 40 && cellHeight > 30) {
        ctx.fillStyle = Math.abs(value) > 0.5 ? '#000' : '#fff'
        ctx.font = '11px monospace'
        ctx.textAlign = 'center'
        ctx.textBaseline = 'middle'
        ctx.fillText(value.toFixed(2), x + cellWidth / 2, y + cellHeight / 2)
      }
    }
  }
}

function scheduleDraw() {
  if (animationId) cancelAnimationFrame(animationId)
  animationId = requestAnimationFrame(() => {
    drawHeatmap()
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
  () => props.data,
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
