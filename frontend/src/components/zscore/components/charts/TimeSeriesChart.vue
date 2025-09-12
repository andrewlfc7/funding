<template>
  <div class="time-series-chart-container" :style="{ height: `${height}px` }">
    <canvas ref="chartCanvas"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted, computed, nextTick } from 'vue'
import { ChartJS } from '@/utils/chartSetup'
import type { ChartConfiguration } from 'chart.js'

interface Props {
  // Single series data
  data?: Array<{
    timestamp: number
    [key: string]: number | string
  }>
  // Multi-series data
  series?: Array<{
    symbol: string
    data: Array<{
      timestamp: number
      value: number
    }>
  }>
  yField?: string
  yLabel?: string
  secondaryYField?: string
  label?: string
  secondaryLabel?: string
  showThresholdLines?: boolean
  thresholds?: number[]
  thresholdLabels?: string[]
  percentileBands?: number[]
  groupBy?: string // Field to group by for multi-series from flat data
  height?: number
  yFormat?: (value: number) => string
  secondaryYFormat?: (value: number) => string
}

const props = withDefaults(defineProps<Props>(), {
  showThresholdLines: false,
  thresholds: () => [],
  thresholdLabels: () => [],
  percentileBands: () => [],
  height: 300
})

const chartCanvas = ref<HTMLCanvasElement>()
let chart: ChartJS | null = null

const colors = [
  'rgba(99, 102, 241, 1)',
  'rgba(236, 72, 153, 1)',
  'rgba(34, 197, 94, 1)',
  'rgba(251, 146, 60, 1)',
  'rgba(147, 51, 234, 1)',
  'rgba(14, 165, 233, 1)',
  'rgba(234, 179, 8, 1)',
  'rgba(239, 68, 68, 1)',
  'rgba(16, 185, 129, 1)',
  'rgba(245, 158, 11, 1)'
]

// ---------- Data shaping (pure/computed) ----------

// Primary/secondary series from flat data
const processedData = computed(() => {
  if (!props.data || props.data.length === 0) return { primary: [] as Array<{timestamp:number; value:number}>, secondary: [] as Array<{timestamp:number; value:number}> }

  const primary = props.yField
    ? props.data.map(d => ({ timestamp: d.timestamp, value: Number(d[props.yField!]) || 0 }))
    : []

  const secondary = props.secondaryYField
    ? props.data.map(d => ({ timestamp: d.timestamp, value: Number(d[props.secondaryYField!]) || 0 }))
    : []

  return { primary, secondary }
})

// Multi-line series (either passed directly, or grouped from flat data)
const processedSeries = computed(() => {
  if (props.series && props.series.length > 0) {
    // Use as-is (assumed immutable from caller)
    return props.series
  }
  if (props.data && props.groupBy && props.yField && !props.secondaryYField) {
    const grouped = new Map<string, Array<{ timestamp: number; value: number }>>()
    for (const item of props.data) {
      const key = String(item[props.groupBy!])
      let arr = grouped.get(key)
      if (!arr) {
        arr = []
        grouped.set(key, arr)
      }
      arr.push({ timestamp: item.timestamp, value: Number(item[props.yField!]) || 0 })
    }
    return Array.from(grouped.entries()).map(([symbol, data]) => ({
      symbol,
      data: data.sort((a, b) => a.timestamp - b.timestamp)
    }))
  }
  return [] as Array<{ symbol: string; data: Array<{ timestamp: number; value: number }> }>
})

// ---------- Small helpers (PURE; no reactive writes) ----------

function getTimeRangeSafe(): [number, number] | null {
  // Avoid Math.min(...bigArray)/max which can overflow the call stack
  let min = Number.POSITIVE_INFINITY
  let max = Number.NEGATIVE_INFINITY

  if (processedData.value.primary.length > 0) {
    for (const d of processedData.value.primary) {
      const t = d.timestamp
      if (Number.isFinite(t)) {
        if (t < min) min = t
        if (t > max) max = t
      }
    }
  } else if (processedSeries.value.length > 0) {
    for (const s of processedSeries.value) {
      for (const d of s.data) {
        const t = d.timestamp
        if (Number.isFinite(t)) {
          if (t < min) min = t
          if (t > max) max = t
        }
      }
    }
  } else {
    return null
  }

  if (!Number.isFinite(min) || !Number.isFinite(max)) return null
  return [min, max]
}

function getThresholdColor(threshold: number): string {
  if (threshold === 0) return 'rgba(255, 255, 255, 0.4)'
  if (Math.abs(threshold) >= 2) return 'rgba(239, 68, 68, 0.5)'
  if (Math.abs(threshold) >= 1) return 'rgba(251, 146, 60, 0.5)'
  return 'rgba(255, 255, 255, 0.2)'
}

// Build datasets (pure; doesn’t mutate props or refs)
function buildDatasets() {
  const datasets: any[] = []
  const useSecondaryAxis = !!(props.secondaryYField && processedData.value.secondary.length > 0)

  if (useSecondaryAxis) {
    datasets.push({
      label: props.label || props.yField || 'Value',
      data: processedData.value.primary.map(d => ({ x: d.timestamp, y: d.value })),
      borderColor: colors[0],
      backgroundColor: colors[0].replace('1)', '0.1)'),
      borderWidth: 2,
      pointRadius: 0,
      tension: 0.1,
      fill: false,
      yAxisID: 'y'
    })
    datasets.push({
      label: props.secondaryLabel || props.secondaryYField,
      data: processedData.value.secondary.map(d => ({ x: d.timestamp, y: d.value })),
      borderColor: colors[1],
      backgroundColor: colors[1].replace('1)', '0.1)'),
      borderWidth: 2,
      pointRadius: 0,
      tension: 0.1,
      fill: false,
      yAxisID: 'y1'
    })
  } else if (processedSeries.value.length > 0) {
    processedSeries.value.forEach((series, index) => {
      datasets.push({
        label: series.symbol,
        data: series.data.map(d => ({ x: d.timestamp, y: d.value })),
        borderColor: colors[index % colors.length],
        backgroundColor: colors[index % colors.length].replace('1)', '0.1)'),
        borderWidth: 2,
        pointRadius: 0,
        tension: 0.1,
        fill: false
      })
    })
  } else if (processedData.value.primary.length > 0) {
    datasets.push({
      label: props.label || props.yField || 'Value',
      data: processedData.value.primary.map(d => ({ x: d.timestamp, y: d.value })),
      borderColor: colors[0],
      backgroundColor: colors[0].replace('1)', '0.1)'),
      borderWidth: 2,
      pointRadius: 0,
      tension: 0.1,
      fill: false
    })
  }

  // Threshold lines (two points: [minTs, maxTs])
  if (props.showThresholdLines && props.thresholds && props.thresholds.length > 0) {
    const range = getTimeRangeSafe()
    if (range) {
      const [tmin, tmax] = range
      for (let i = 0; i < props.thresholds.length; i++) {
        const threshold = props.thresholds[i]
        datasets.push({
          label: props.thresholdLabels?.[i] || `${threshold}σ`,
          data: [
            { x: tmin, y: threshold },
            { x: tmax, y: threshold }
          ],
          borderColor: getThresholdColor(threshold),
          borderWidth: threshold === 0 ? 2 : 1,
          borderDash: [5, 5],
          pointRadius: 0,
          fill: false,
          yAxisID: 'y'
        })
      }
    }
  }

  return { datasets, useSecondaryAxis }
}

// ---------- Chart lifecycle ----------

function createChart() {
  if (!chartCanvas.value) return

  const { datasets, useSecondaryAxis } = buildDatasets()

  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: { datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      spanGaps: true,        // perf: skip nulls without segmenting
      animation: false,      // perf: disable animations for large data
      parsing: false,        // perf: we already feed {x,y}
      interaction: {
        mode: 'index',
        intersect: false
      },
      plugins: {
        legend: {
          display: datasets.length > 1,
          position: 'top',
          labels: {
            usePointStyle: true,
            padding: 10,
            font: { size: 12 },
            filter: (item) => {
              // Hide threshold lines from legend if too many series
              if (item.text?.includes('σ') && datasets.length > 5) return false
              return true
            }
          }
        },
        tooltip: {
          mode: 'index',
          intersect: false,
          callbacks: {
            label: (context) => {
              // Skip tooltips for dashed (threshold) series
              if ((context.dataset as any).borderDash) return ''
              const label = context.dataset.label || ''
              const y = Number(context.parsed.y)
              if (context.dataset.yAxisID === 'y' && props.yFormat) {
                return `${label}: ${props.yFormat(y)}`
              } else if (context.dataset.yAxisID === 'y1' && props.secondaryYFormat) {
                return `${label}: ${props.secondaryYFormat(y)}`
              }
              return `${label}: ${y.toFixed(3)}`
            }
          }
        }
      },
      scales: {
        x: {
          type: 'time',
          time: {
            // Heuristic default; Chart.js will choose nice ticks for wide ranges
            unit: 'hour',
            displayFormats: {
              hour: 'HH:mm',
              day: 'MMM dd'
            }
          },
          grid: { color: 'rgba(255, 255, 255, 0.05)' },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)',
            maxRotation: 0
          }
        },
        y: {
          position: 'left',
          grid: { color: 'rgba(255, 255, 255, 0.05)' },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)',
            callback(value) {
              if (props.yFormat) return props.yFormat(Number(value))
              return String(value)
            }
          },
          title: {
            display: !!props.label,
            text: props.label,
            color: 'rgba(255, 255, 255, 0.8)'
          }
        },
        ...(useSecondaryAxis
          ? {
              y1: {
                position: 'right',
                grid: { drawOnChartArea: false },
                ticks: {
                  color: 'rgba(255, 255, 255, 0.6)',
                  callback(value) {
                    if (props.secondaryYFormat) return props.secondaryYFormat(Number(value))
                    return String(value)
                  }
                },
                title: {
                  display: !!props.secondaryLabel,
                  text: props.secondaryLabel,
                  color: 'rgba(255, 255, 255, 0.8)'
                }
              }
            }
          : {})
      }
    }
  }

  chart = new ChartJS(chartCanvas.value.getContext('2d')!, config)
}

function updateChart() {
  if (!chart) {
    createChart()
    return
  }
  const { datasets, useSecondaryAxis } = buildDatasets()

  // Replace datasets & (conditionally) the secondary axis
  chart.data.datasets = datasets as any

  // Ensure axis definitions are in sync
  if (useSecondaryAxis) {
    chart.options.scales!.y1 = chart.options.scales!.y1 || {
      position: 'right',
      grid: { drawOnChartArea: false }
    }
  } else {
    if (chart.options.scales && chart.options.scales.y1) {
      delete chart.options.scales.y1
    }
  }

  chart.update('none') // perf: no animation
}

// ---------- Mount / Watch / Unmount ----------

onMounted(async () => {
  await nextTick()
  createChart()
})

// Important: no deep watch — assume callers replace arrays immutably.
// This avoids noisy reactivity and accidental loops.
watch(
  () => [props.data, props.series, props.yField, props.secondaryYField, props.groupBy, props.showThresholdLines, props.thresholds],
  async () => {
    await nextTick()
    updateChart()
  },
  { deep: false }
)

onUnmounted(() => {
  if (chart) {
    chart.destroy()
    chart = null
  }
})
</script>

<style scoped>
.time-series-chart-container {
  position: relative;
  width: 100%;
}

.time-series-chart-container canvas {
  width: 100% !important;
  height: 100% !important;
}
</style>
