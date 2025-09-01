<!-- src/components/trend/CombinedSignalsChart.vue -->
<template>
  <div class="chart-wrapper subplot-chart-wrapper">
    <canvas ref="canvasEl" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import Chart from 'chart.js/auto'
import 'chartjs-adapter-date-fns'
import { xsecSignals, type XsecSignal } from '@/utils/xsecSignals'

interface Props {
  labels: Date[]
  dataById: Record<string, number[]>
  signals?: XsecSignal[]
  activeSignals?: Record<string, boolean>
  loading?: boolean
  error?: string | null
}
const props = withDefaults(defineProps<Props>(), {
  signals: () => xsecSignals,
  activeSignals: () => ({ momentum: true, ewmac: true, breakout: true, composite: true })
})

const canvasEl = ref<HTMLCanvasElement | null>(null)
let chart: Chart<'line'> | null = null

const datasets = computed(() => {
  const ds: any[] = []
  for (const s of props.signals!) {
    if (props.activeSignals && props.activeSignals[s.id] === false) continue
    ds.push({
      label: s.name,
      borderColor: s.color,
      data: (props.dataById[s.id] ?? []).map((y, i) => ({ x: props.labels[i], y })),
      borderWidth: 1,
      pointRadius: 0,
    })
  }
  return ds
})

function buildConfig() {
  return {
    type: 'line' as const,
    data: { datasets: datasets.value },
    options: {
      parsing: false as const,
      animation: false,
      maintainAspectRatio: false,
      normalized: true,
      plugins: {
        legend: { display: true },
        tooltip: { mode: 'index', intersect: false },
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
  chart.data.datasets = datasets.value
  chart.update('none')
}

onMounted(() => { createChart(); updateChart() })
onUnmounted(() => { chart?.destroy(); chart = null })
watch(() => [props.labels, props.dataById, props.activeSignals], updateChart, { deep: true })
</script>

<style scoped>
.subplot-chart-wrapper { position: relative; width: 100%; min-height: 260px; }
</style>
