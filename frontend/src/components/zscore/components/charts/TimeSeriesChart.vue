<template>
  <div class="time-series-chart-container">
    <canvas ref="chartCanvas"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted, computed } from 'vue'
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
}

const props = defineProps<Props>()
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

// Process data into series
const processedSeries = computed(() => {
  // If series prop is provided, use it directly
  if (props.series && props.series.length > 0) {
    return props.series
  }
  
  // If groupBy is specified, group the flat data
  if (props.data && props.groupBy && props.yField) {
    const grouped = new Map<string, Array<{timestamp: number, value: number}>>()
    
    props.data.forEach(item => {
      const key = String(item[props.groupBy])
      if (!grouped.has(key)) {
        grouped.set(key, [])
      }
      grouped.get(key)!.push({
        timestamp: item.timestamp,
        value: item[props.yField] as number
      })
    })
    
    return Array.from(grouped.entries()).map(([symbol, data]) => ({
      symbol,
      data: data.sort((a, b) => a.timestamp - b.timestamp)
    }))
  }
  
  // Default to single series
  if (props.data && props.yField) {
    return [{
      symbol: props.label || props.yField,
      data: props.data.map(d => ({
        timestamp: d.timestamp,
        value: d[props.yField] as number
      }))
    }]
  }
  
  return []
})

function createChart() {
  if (!chartCanvas.value || processedSeries.value.length === 0) return

  const datasets: any[] = []
  
  // Create dataset for each series
  processedSeries.value.forEach((series, index) => {
    datasets.push({
      label: series.symbol,
      data: series.data.map(d => ({
        x: d.timestamp,
        y: d.value
      })),
      borderColor: colors[index % colors.length],
      backgroundColor: colors[index % colors.length].replace('1)', '0.1)'),
      borderWidth: 2,
      pointRadius: 0,
      tension: 0.1,
      fill: false
    })
  })

  // Add threshold lines
  if (props.showThresholdLines && props.thresholds) {
    props.thresholds.forEach((threshold, index) => {
      datasets.push({
        label: props.thresholdLabels?.[index] || `${threshold}σ`,
        data: getTimeRange().map(timestamp => ({
          x: timestamp,
          y: threshold
        })),
        borderColor: getThresholdColor(threshold),
        borderWidth: threshold === 0 ? 2 : 1,
        borderDash: [5, 5],
        pointRadius: 0,
        fill: false,
        showLine: true
      })
    })
  }

  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: { datasets },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: {
        mode: 'index',
        intersect: false
      },
      plugins: {
        legend: {
          display: processedSeries.value.length > 1 || props.showThresholdLines,
          position: 'top',
          labels: {
            usePointStyle: true,
            padding: 10,
            font: {
              size: 12
            },
            filter: (item) => {
              // Hide threshold lines from legend if too many series
              if (item.text?.includes('σ') && processedSeries.value.length > 5) {
                return false
              }
              return true
            }
          }
        },
        tooltip: {
          mode: 'index',
          intersect: false,
          callbacks: {
            label: (context) => {
              // Don't show tooltip for threshold lines
              if (context.dataset.borderDash) return null
              
              const label = context.dataset.label || ''
              const value = context.parsed.y.toFixed(3)
              return `${label}: ${value}`
            }
          }
        }
      },
      scales: {
        x: {
          type: 'time',
          time: {
            unit: 'hour',
            displayFormats: {
              hour: 'HH:mm',
              day: 'MMM dd'
            }
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)',
            maxRotation: 0
          }
        },
        y: {
          position: 'left',
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)'
          },
          title: {
            display: !!props.yLabel,
            text: props.yLabel,
            color: 'rgba(255, 255, 255, 0.8)'
          }
        }
      }
    }
  }

  chart = new ChartJS(chartCanvas.value.getContext('2d')!, config)
}

function getTimeRange(): number[] {
  const allTimestamps = processedSeries.value.flatMap(s => s.data.map(d => d.timestamp))
  const min = Math.min(...allTimestamps)
  const max = Math.max(...allTimestamps)
  return [min, max]
}

function getThresholdColor(threshold: number): string {
  if (threshold === 0) return 'rgba(255, 255, 255, 0.4)'
  if (Math.abs(threshold) >= 2) return 'rgba(239, 68, 68, 0.5)'
  if (Math.abs(threshold) >= 1) return 'rgba(251, 146, 60, 0.5)'
  return 'rgba(255, 255, 255, 0.2)'
}

function updateChart() {
  if (!chart || processedSeries.value.length === 0) return

  // Clear existing datasets
  chart.data.datasets = []

  // Re-add series datasets
  processedSeries.value.forEach((series, index) => {
    chart!.data.datasets.push({
      label: series.symbol,
      data: series.data.map(d => ({
        x: d.timestamp,
        y: d.value
      })),
      borderColor: colors[index % colors.length],
      backgroundColor: colors[index % colors.length].replace('1)', '0.1)'),
      borderWidth: 2,
      pointRadius: 0,
      tension: 0.1,
      fill: false
    })
  })

  // Re-add threshold lines
  if (props.showThresholdLines && props.thresholds) {
    props.thresholds.forEach((threshold, index) => {
      chart!.data.datasets.push({
        label: props.thresholdLabels?.[index] || `${threshold}σ`,
        data: getTimeRange().map(timestamp => ({
          x: timestamp,
          y: threshold
        })),
        borderColor: getThresholdColor(threshold),
        borderWidth: threshold === 0 ? 2 : 1,
        borderDash: [5, 5],
        pointRadius: 0,
        fill: false,
        showLine: true
      })
    })
  }

  chart.update('none')
}

onMounted(() => {
  createChart()
})

watch(() => [props.data, props.series, props.yField, props.groupBy], () => {
  if (chart) {
    updateChart()
  } else {
    createChart()
  }
}, { deep: true })

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
  height: 100%;
  min-height: 250px;
}

.time-series-chart-container canvas {
  width: 100% !important;
  height: 100% !important;
}
</style>