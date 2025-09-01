<!-- src/components/trend/ReturnsVolatilityChart.vue -->
<template>
  <div class="returns-volatility-container">
    <div class="returns-volatility-grid">
      <!-- Returns Chart -->
      <div class="rv-chart-wrapper">
        <h3 class="rv-chart-title">Returns</h3>
        <div class="rv-chart-canvas-container">
          <canvas ref="returnsCanvasEl" />
        </div>
      </div>
      
      <!-- Volatility Chart -->
      <div class="rv-chart-wrapper">
        <h3 class="rv-chart-title">Volatility</h3>
        <div class="rv-chart-canvas-container">
          <canvas ref="volatilityCanvasEl" />
        </div>
      </div>
    </div>
    
    <!-- Loading/Error states -->
    <div v-if="loading" class="rv-overlay-message">Loading...</div>
    <div v-else-if="error" class="rv-overlay-message error">{{ error }}</div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import Chart from 'chart.js/auto'
import 'chartjs-adapter-date-fns'
import type { ChartDataset, ScatterDataPoint } from 'chart.js'

interface Point { ts: number; value: number }

interface Props {
  returns: Point[]
  volatility: Point[]
  loading?: boolean
  error?: string | null
}

const props = defineProps<Props>()

const returnsCanvasEl = ref<HTMLCanvasElement | null>(null)
const volatilityCanvasEl = ref<HTMLCanvasElement | null>(null)
let returnsChart: Chart<'line', ScatterDataPoint[]> | null = null
let volatilityChart: Chart<'line', ScatterDataPoint[]> | null = null

function mapToXY(arr: Point[]): ScatterDataPoint[] {
  return arr.map(p => ({ x: p.ts, y: p.value }))
}

function createChartConfig(data: Point[], label: string, color: string) {
  return {
    type: 'line' as const,
    data: {
      datasets: [{
        label,
        data: mapToXY(data),
        borderColor: color,
        backgroundColor: color + '20',
        borderWidth: 2,
        pointRadius: 0,
        tension: 0.1,
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      parsing: false,
      animation: false,
      normalized: true,
      plugins: {
        legend: { display: false },
        tooltip: {
          mode: 'index' as const,
          intersect: false,
          callbacks: {
            label: (context: any) => {
              const value = context.parsed.y
              return `${label}: ${value.toFixed(4)}`
            }
          }
        },
      },
      scales: {
        x: {
          type: 'time',
          time: { unit: 'day' },
          ticks: { 
            maxRotation: 0, 
            autoSkip: true,
            color: '#666'
          },
          grid: {
            display: false // Remove grid lines
          }
        },
        y: { 
          beginAtZero: false,
          ticks: {
            color: '#666',
            callback: (value: any) => value.toFixed(4)
          },
          grid: {
            display: false // Remove grid lines
          }
        },
      },
    },
  }
}

function createCharts() {
  // Create Returns Chart
  if (returnsCanvasEl.value && !returnsChart) {
    const ctx = returnsCanvasEl.value.getContext('2d')
    if (ctx) {
      returnsChart = new Chart(ctx, createChartConfig(
        props.returns, 
        'Returns', 
        '#3b82f6' // blue
      ) as any)
    }
  }
  
  // Create Volatility Chart
  if (volatilityCanvasEl.value && !volatilityChart) {
    const ctx = volatilityCanvasEl.value.getContext('2d')
    if (ctx) {
      volatilityChart = new Chart(ctx, createChartConfig(
        props.volatility, 
        'Volatility', 
        '#10b981' // green
      ) as any)
    }
  }
}

function updateCharts() {
  // Update Returns Chart
  if (returnsChart) {
    returnsChart.data.datasets[0].data = mapToXY(props.returns)
    returnsChart.update('none')
  } else {
    createCharts()
  }
  
  // Update Volatility Chart
  if (volatilityChart) {
    volatilityChart.data.datasets[0].data = mapToXY(props.volatility)
    volatilityChart.update('none')
  }
}

onMounted(() => {
  createCharts()
  updateCharts()
})

onUnmounted(() => {
  returnsChart?.destroy()
  volatilityChart?.destroy()
  returnsChart = null
  volatilityChart = null
})

watch(() => [props.returns, props.volatility], updateCharts, { deep: true })
</script>