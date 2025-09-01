<!-- src/components/trend/CandlestickChart.vue -->
<template>
  <div class="chart-wrapper main-chart-wrapper">
    <canvas ref="canvasEl" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'

import { ChartJS } from '@/utils/chartSetup'

import type { KlineDTO } from '@/utils/types'
import type { ChartConfiguration } from 'chart.js'

interface Props {
  data: {
    klines?: KlineDTO[]
    loading?: boolean
    error?: string | null
  }
  coin: string
  period: string
}

const props = defineProps<Props>()

const canvasEl = ref<HTMLCanvasElement | null>(null)

const candleData = computed(() => {
  const rows = props.data?.klines || []
  // financial plugin wants {x, o, h, l, c} with x as Date or timestamp (ms)
  return rows.map(k => ({
    x: k.ts,                // epoch ms (number) is fine with the adapter
    o: k.open,
    h: k.high,
    l: k.low,
    c: k.close,
  }))
})


function buildConfig(): ChartConfiguration<'candlestick', {x: number, o: number, h: number, l: number, c: number}[], unknown> {
  return {
    type: 'candlestick',
    data: {
      datasets: [
        {
          label: props.coin,
          data: candleData.value,
        },
      ],
    },
    options: {
      parsing: false as const,  // ✅ fixes type error
      animation: false,
      maintainAspectRatio: false,
      normalized: true,
      plugins: {
        legend: { display: false },
        tooltip: { mode: 'index', intersect: false },
      },
      scales: {
        x: { type: 'time', time: { unit: 'day' } },
        y: { beginAtZero: false },
      },
    },
  }
}



let chart: ChartJS<'candlestick'> | null = null

function createChart() {
  if (!canvasEl.value) return
  if (chart) return
  chart = new ChartJS(canvasEl.value.getContext('2d')!, buildConfig())
}

function updateChart() {
  if (!chart) {
    createChart()
    return
  }
  // Update dataset in place; do NOT new Chart(...)
  chart.data.datasets[0].data = candleData.value as any
  // Optionally update label if coin changed
  chart.data.datasets[0].label = props.coin
  chart.update('none')
}

onMounted(() => {
  createChart()
  updateChart()
})

onUnmounted(() => {
  if (chart) {
    chart.destroy()
    chart = null
  }
})

// Deep-watch the input data and coin label
watch(
  () => [props.data?.klines, props.coin],
  () => {
    updateChart()
  },
  { deep: true }
)
</script>

<style scoped>
.main-chart-wrapper {
  position: relative;
  width: 100%;
  min-height: 360px;
}
</style>


