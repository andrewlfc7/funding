<template>
  <div class="heatmap-chart-container" ref="containerRef">
    <canvas ref="chartCanvas"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted, nextTick } from 'vue'

interface Props {
  data: {
    labels: string[]
    data: number[][]
  }
  min?: number
  max?: number
  colorScheme?: 'correlation' | 'beta' | 'default'
}

const props = withDefaults(defineProps<Props>(), {
  min: 0,
  max: 1,
  colorScheme: 'default'
})

const chartCanvas = ref<HTMLCanvasElement>()
const containerRef = ref<HTMLDivElement>()
let resizeObserver: ResizeObserver | null = null
let animationFrameId: number | null = null

function getColorForValue(value: number): string {
  const normalized = (value - props.min) / (props.max - props.min)
  
  if (props.colorScheme === 'correlation') {
    // Red to green for correlations
    if (normalized < 0.5) {
      const intensity = normalized * 2
      return `rgba(239, 68, 68, ${0.2 + intensity * 0.8})`
    } else {
      const intensity = (normalized - 0.5) * 2
      return `rgba(34, 197, 94, ${0.2 + intensity * 0.8})`
    }
  } else if (props.colorScheme === 'beta') {
    // Blue to purple for beta
    const r = 59 + (139 - 59) * normalized
    const g = 130 - (130 - 92) * normalized
    const b = 246 - (246 - 233) * normalized
    return `rgba(${r}, ${g}, ${b}, 0.8)`
  } else {
    // Default gradient
    return `rgba(99, 102, 241, ${0.2 + normalized * 0.8})`
  }
}

function drawHeatmap() {
  if (!chartCanvas.value || !containerRef.value || !props.data) return

  const ctx = chartCanvas.value.getContext('2d')!
  const { labels, data } = props.data
  
  // Get container dimensions
  const containerWidth = containerRef.value.clientWidth
  const containerHeight = containerRef.value.clientHeight
  
  // Set canvas size to match container
  chartCanvas.value.width = containerWidth
  chartCanvas.value.height = containerHeight
  
  // Calculate cell dimensions
  const padding = 40 // Space for labels
  const availableWidth = containerWidth - padding
  const availableHeight = containerHeight - padding
  
  const cellWidth = availableWidth / labels.length
  const cellHeight = availableHeight / data.length
  
  // Clear canvas
  ctx.clearRect(0, 0, containerWidth, containerHeight)
  
  // Draw cells
  data.forEach((row, i) => {
    row.forEach((value, j) => {
      const x = j * cellWidth
      const y = i * cellHeight
      
      ctx.fillStyle = getColorForValue(value)
      ctx.fillRect(x, y, cellWidth - 1, cellHeight - 1)
      
      // Add text labels for small matrices
      if (labels.length <= 10) {
        ctx.save()
        ctx.fillStyle = normalized > 0.5 ? '#000' : '#fff'
        ctx.font = '11px sans-serif'
        ctx.textAlign = 'center'
        ctx.textBaseline = 'middle'
        ctx.fillText(value.toFixed(2), x + cellWidth / 2, y + cellHeight / 2)
        ctx.restore()
      }
    })
  })
  
  // Draw labels
  ctx.save()
  ctx.fillStyle = '#999'
  ctx.font = '11px sans-serif'
  
  // X-axis labels
  labels.forEach((label, i) => {
    const x = i * cellWidth + cellWidth / 2
    const y = availableHeight + 20
    
    ctx.save()
    ctx.translate(x, y)
    ctx.rotate(-Math.PI / 4)
    ctx.textAlign = 'right'
    ctx.textBaseline = 'middle'
    ctx.fillText(label, 0, 0)
    ctx.restore()
  })
  
  // Y-axis labels (for square matrices)
  if (data.length === labels.length) {
    labels.forEach((label, i) => {
      const x = availableWidth + 10
      const y = i * cellHeight + cellHeight / 2
      
      ctx.textAlign = 'left'
      ctx.textBaseline = 'middle'
      ctx.fillText(label, x, y)
    })
  }
  
  ctx.restore()
}

function handleResize() {
  // Cancel any pending animation frame
  if (animationFrameId) {
    cancelAnimationFrame(animationFrameId)
  }
  
  // Schedule redraw on next animation frame
  animationFrameId = requestAnimationFrame(() => {
    drawHeatmap()
  })
}

onMounted(async () => {
  await nextTick()
  
  if (containerRef.value) {
    // Initial draw
    drawHeatmap()
    
    // Set up resize observer
    resizeObserver = new ResizeObserver((entries) => {
      // Only handle resize if size actually changed
      const entry = entries[0]
      if (entry.contentRect.width > 0 && entry.contentRect.height > 0) {
        handleResize()
      }
    })
    
    resizeObserver.observe(containerRef.value)
  }
})

watch(() => props.data, () => {
  handleResize()
}, { deep: true })

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect()
    resizeObserver = null
  }
  
  if (animationFrameId) {
    cancelAnimationFrame(animationFrameId)
  }
})
</script>

<style scoped>
.heatmap-chart-container {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 300px;
  max-height: 500px; /* Prevent infinite growth */
  overflow: hidden;
}

.heatmap-chart-container canvas {
  display: block;
  width: 100%;
  height: 100%;
}
</style>