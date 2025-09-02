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

// Process data for dual y-axis
const processedData = computed(() => {
  if (!props.data || props.data.length === 0) return { primary: [], secondary: [] }
  
  const primary = props.data.map(d => ({
    timestamp: d.timestamp,
    value: props.yField ? (d[props.yField] as number) : 0
  }))
  
  const secondary = props.secondaryYField 
    ? props.data.map(d => ({
        timestamp: d.timestamp,
        value: d[props.secondaryYField] as number
      }))
    : []
  
  return { primary, secondary }
})

// Process data into series (for multi-line without secondary axis)
const processedSeries = computed(() => {
  // If series prop is provided, use it directly
  if (props.series && props.series.length > 0) {
    return props.series
  }
  
  // If groupBy is specified, group the flat data
  if (props.data && props.groupBy && props.yField && !props.secondaryYField) {
    const grouped = new Map<string, Array<{timestamp: number, value: number}>>()
    
    props.data.forEach(item => {
      const key = String(item[props.groupBy!])
      if (!grouped.has(key)) {
        grouped.set(key, [])
      }
      grouped.get(key)!.push({
        timestamp: item.timestamp,
        value: item[props.yField!] as number
      })
    })
    
    return Array.from(grouped.entries()).map(([symbol, data]) => ({
      symbol,
      data: data.sort((a, b) => a.timestamp - b.timestamp)
    }))
  }
  
  return []
})

function createChart() {
  if (!chartCanvas.value) return

  const datasets: any[] = []
  const useSecondaryAxis = props.secondaryYField && processedData.value.secondary.length > 0

  if (useSecondaryAxis) {
    // Dual y-axis mode
    // Primary dataset
    datasets.push({
      label: props.label || props.yField || 'Value',
      data: processedData.value.primary.map(d => ({
        x: d.timestamp,
        y: d.value
      })),
      borderColor: colors[0],
      backgroundColor: colors[0].replace('1)', '0.1)'),
      borderWidth: 2,
      pointRadius: 0,
      tension: 0.1,
      fill: false,
      yAxisID: 'y'
    })
    
    // Secondary dataset
    datasets.push({
      label: props.secondaryLabel || props.secondaryYField,
      data: processedData.value.secondary.map(d => ({
        x: d.timestamp,
        y: d.value
      })),
      borderColor: colors[1],
      backgroundColor: colors[1].replace('1)', '0.1)'),
      borderWidth: 2,
      pointRadius: 0,
      tension: 0.1,
      fill: false,
      yAxisID: 'y1'
    })
  } else if (processedSeries.value.length > 0) {
    // Multi-series mode (single y-axis)
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
  } else if (processedData.value.primary.length > 0) {
    // Single series mode
    datasets.push({
      label: props.label || props.yField || 'Value',
      data: processedData.value.primary.map(d => ({
        x: d.timestamp,
        y: d.value
      })),
      borderColor: colors[0],
      backgroundColor: colors[0].replace('1)', '0.1)'),
      borderWidth: 2,
      pointRadius: 0,
      tension: 0.1,
      fill: false
    })
  }

  // Add threshold lines
  if (props.showThresholdLines && props.thresholds && datasets.length > 0) {
    const timeRange = getTimeRange()
    props.thresholds.forEach((threshold, index) => {
      datasets.push({
        label: props.thresholdLabels?.[index] || `${threshold}σ`,
        data: timeRange.map(timestamp => ({
          x: timestamp,
          y: threshold
        })),
        borderColor: getThresholdColor(threshold),
        borderWidth: threshold === 0 ? 2 : 1,
        borderDash: [5, 5],
        pointRadius: 0,
        fill: false,
        showLine: true,
        yAxisID: 'y'
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
          display: datasets.length > 1,
          position: 'top',
          labels: {
            usePointStyle: true,
            padding: 10,
            font: {
              size: 12
            },
            filter: (item) => {
              // Hide threshold lines from legend if too many series
              if (item.text?.includes('σ') && datasets.length > 5) {
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
              if (context.dataset.borderDash) return ''
              
              const label = context.dataset.label || ''
              let value = context.parsed.y
              
              // Format based on y-axis
              if (context.dataset.yAxisID === 'y' && props.yFormat) {
                value = props.yFormat(value)
              } else if (context.dataset.yAxisID === 'y1' && props.secondaryYFormat) {
                value = props.secondaryYFormat(value)
              } else {
                value = value.toFixed(3)
              }
              
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
            color: 'rgba(255, 255, 255, 0.6)',
            callback: function(value) {
              if (props.yFormat) {
                return props.yFormat(value as number)
              }
              return value
            }
          },
          title: {
            display: !!props.label,
            text: props.label,
            color: 'rgba(255, 255, 255, 0.8)'
          }
        },
        ...(useSecondaryAxis ? {
          y1: {
            position: 'right',
            grid: {
              drawOnChartArea: false
            },
            ticks: {
              color: 'rgba(255, 255, 255, 0.6)',
              callback: function(value) {
                if (props.secondaryYFormat) {
                  return props.secondaryYFormat(value as number)
                }
                return value
              }
            },
            title: {
              display: !!props.secondaryLabel,
              text: props.secondaryLabel,
              color: 'rgba(255, 255, 255, 0.8)'
            }
          }
        } : {})
      }
    }
  }

  chart = new ChartJS(chartCanvas.value.getContext('2d')!, config)
}

function getTimeRange(): number[] {
  if (processedData.value.primary.length > 0) {
    const timestamps = processedData.value.primary.map(d => d.timestamp)
    return [Math.min(...timestamps), Math.max(...timestamps)]
  }
  if (processedSeries.value.length > 0) {
    const allTimestamps = processedSeries.value.flatMap(s => s.data.map(d => d.timestamp))
    return [Math.min(...allTimestamps), Math.max(...allTimestamps)]
  }
  return []
}

function getThresholdColor(threshold: number): string {
  if (threshold === 0) return 'rgba(255, 255, 255, 0.4)'
  if (Math.abs(threshold) >= 2) return 'rgba(239, 68, 68, 0.5)'
  if (Math.abs(threshold) >= 1) return 'rgba(251, 146, 60, 0.5)'
  return 'rgba(255, 255, 255, 0.2)'
}

function updateChart() {
  if (!chart) return

  // Destroy and recreate for significant changes
  chart.destroy()
  createChart()
}

onMounted(() => {
  createChart()
})

watch(() => [props.data, props.series, props.yField, props.secondaryYField, props.groupBy], () => {
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
}

.time-series-chart-container canvas {
  width: 100% !important;
  height: 100% !important;
}
</style>