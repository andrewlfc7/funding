<template>
  <div class="histogram-chart-container">
    <canvas ref="chartCanvas"></canvas>
    <div v-if="currentValue !== undefined" class="current-value">
      Current: {{ currentValue.toFixed(2) }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted } from 'vue'
import { ChartJS } from '@/utils/chartSetup'
import type { ChartConfiguration } from 'chart.js'

interface Props {
  data: {
    buckets: number[] | string[]
    counts: number[]
  }
  currentValue?: number
  highlights?: Array<{ value: number; label: string }>
}

const props = defineProps<Props>()
const chartCanvas = ref<HTMLCanvasElement>()
let chart: ChartJS | null = null

function createChart() {
  if (!chartCanvas.value || !props.data) return

  const config: ChartConfiguration<'bar'> = {
    type: 'bar',
    data: {
      labels: props.data.buckets.map(b => b.toString()),
      datasets: [{
        label: 'Frequency',
        data: props.data.counts,
        backgroundColor: props.data.buckets.map((_, i) => {
          // Color extreme buckets differently
          if (typeof props.data.buckets[0] === 'number') {
            const value = props.data.buckets[i] as number
            if (value < -2) return 'rgba(239, 68, 68, 0.8)'  // Red for very negative
            if (value > 2) return 'rgba(34, 197, 94, 0.8)'   // Green for very positive
          }
          return 'rgba(99, 102, 241, 0.8)'
        }),
        borderColor: 'rgba(99, 102, 241, 1)',
        borderWidth: 1
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      plugins: {
        legend: {
          display: false
        },
        tooltip: {
          callbacks: {
            afterLabel: (context) => {
              // Show which coins are in this bucket if highlights are provided
              if (props.highlights && context.dataIndex !== undefined) {
                const bucket = props.data.buckets[context.dataIndex]
                const relevantHighlights = props.highlights.filter(h => {
                  if (typeof bucket === 'number' && context.dataIndex < props.data.buckets.length - 1) {
                    const nextBucket = props.data.buckets[context.dataIndex + 1] as number
                    return h.value >= bucket && h.value < nextBucket
                  }
                  return false
                })
                
                if (relevantHighlights.length > 0) {
                  return relevantHighlights.map(h => h.label).join(', ')
                }
              }
              return ''
            }
          }
        }
      },
      scales: {
        x: {
          title: {
            display: true,
            text: typeof props.data.buckets[0] === 'number' ? 'Z-Score' : 'Range',
            color: 'rgba(255, 255, 255, 0.8)'
          },
          grid: {
            display: false
          },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)'
          }
        },
        y: {
                    title: {
            display: true,
            text: 'Count',
            color: 'rgba(255, 255, 255, 0.8)'
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)'
          }
        }
      }
    }
  }

  chart = new ChartJS(chartCanvas.value.getContext('2d')!, config)
}

function updateChart() {
  if (!chart || !props.data) return

  chart.data.labels = props.data.buckets.map(b => b.toString())
  chart.data.datasets[0].data = props.data.counts
  
  // Update colors based on values
  if (chart.data.datasets[0].backgroundColor) {
    (chart.data.datasets[0].backgroundColor as string[]) = props.data.buckets.map((_, i) => {
      if (typeof props.data.buckets[0] === 'number') {
        const value = props.data.buckets[i] as number
        if (value < -2) return 'rgba(239, 68, 68, 0.8)'
        if (value > 2) return 'rgba(34, 197, 94, 0.8)'
      }
      return 'rgba(99, 102, 241, 0.8)'
    })
  }
  
  chart.update('none')
}

onMounted(() => {
  createChart()
})

watch(() => [props.data, props.highlights], () => {
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
.histogram-chart-container {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 250px;
}

.current-value {
  position: absolute;
  top: 10px;
  right: 10px;
  background: var(--bg-tertiary);
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 13px;
  color: var(--text-primary);
  font-weight: 500;
}
</style>
        