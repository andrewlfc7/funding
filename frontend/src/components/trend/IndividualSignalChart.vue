<!-- src/components/trend/IndividualSignalChart.vue -->
<template>
  <div class="chart-wrapper subplot-chart-wrapper">
    <canvas ref="canvasEl" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import Chart from 'chart.js/auto'
import 'chartjs-adapter-date-fns'
import type { ChartDataset, ScatterDataPoint } from 'chart.js'

interface Props {
  title: string
  color: string
  labels: Date[]        // x values
  values: number[]      // y values
  loading?: boolean
  error?: string | null
}
const props = defineProps<Props>()

const canvasEl = ref<HTMLCanvasElement | null>(null)
let chart: Chart<'line', ScatterDataPoint[]> | null = null

const dataset = computed<ChartDataset<'line', ScatterDataPoint[]>[]>(() => [{
  label: props.title,
  borderColor: props.color,
  data: props.values.map((y, i) => ({
    // convert Date -> timestamp to keep the scale happy
    x: props.labels[i] instanceof Date ? props.labels[i].getTime() : Number(props.labels[i]),
    y,
  })),
  borderWidth: 1,
  pointRadius: 0,
}])

function buildConfig() {
  return {
    type: 'line' as const,
    data: { datasets: dataset.value },
    options: {
      parsing: false as const, // we supply {x,y}
      animation: false,
      maintainAspectRatio: false,
      normalized: true,
      plugins: {
        legend: { display: true },
        tooltip: { mode: 'index', intersect: false }
      },
      scales: {
        x: { type: 'time', time: { unit: 'day' }, ticks: { maxRotation: 0, autoSkip: true } },
        y: { beginAtZero: false, title: { display: true, text: 'Z-Score' } },
      },
    },
  }
}

function createChart() {
  if (!canvasEl.value || chart) return
  const ctx = canvasEl.value.getContext('2d')  
  if (!ctx) return
  chart = new Chart(ctx, buildConfig() as any)
}
function updateChart() {
  if (!chart) { createChart(); return }
  chart.data.datasets = dataset.value
  chart.update('none')
}

onMounted(() => { createChart(); updateChart() })
onUnmounted(() => { chart?.destroy(); chart = null })
watch(() => [props.labels, props.values], updateChart, { deep: true })
</script>

<style scoped>
.subplot-chart-wrapper { position: relative; width: 100%; min-height: 220px; }
</style>
