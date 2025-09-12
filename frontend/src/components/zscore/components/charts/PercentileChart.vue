<template>
  <div class="percentile-chart-container">
    <canvas ref="chartCanvas"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted } from 'vue'
import { ChartJS } from '@/utils/chartSetup'
import type { ChartConfiguration } from 'chart.js'

interface Props {
  data: Array<{
    symbol: string
    percentile: number
    volatility: number
    volatilityZScore: number
  }>
}

const props = defineProps<Props>()
const chartCanvas = ref<HTMLCanvasElement>()
let chart: ChartJS | null = null

function createChart() {
  if (!chartCanvas.value || !props.data.length) return

  const sortedData = [...props.data].sort((a, b) => a.percentile - b.percentile)
  
  const config: ChartConfiguration<'line'> = {
    type: 'line',
    data: {
      labels: sortedData.map(d => d.symbol),
      datasets: [{
        label: 'Volatility Percentile',
        data: sortedData.map(d => d.percentile),
        borderColor: 'rgba(99, 102, 241, 1)',
        backgroundColor: 'rgba(99, 102, 241, 0.1)',
        borderWidth: 2,
        fill: true,
        tension: 0.4,
        pointRadius: 4,
        pointHoverRadius: 6,
        pointBackgroundColor: (context) => {
          const value = context.parsed.y
          if (value > 80) return '#ef4444' // Red for high percentiles
          if (value < 20) return '#22c55e' // Green for low percentiles
          return 'rgba(99, 102, 241, 1)'
        }
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
            title: (items) => {
              const item = sortedData[items[0].dataIndex]
              return item.symbol
            },
            label: (context) => {
              const item = sortedData[context.dataIndex]
              return [
                `Percentile: ${item.percentile.toFixed(0)}%`,
                `Volatility: ${item.volatility.toFixed(1)}%`,
                `Z-Score: ${item.volatilityZScore.toFixed(2)}σ`
              ]
            }
          }
        },
      },
      scales: {
        x: {
          display: false // Hide x-axis labels (too many symbols)
        },
        y: {
          title: {
            display: true,
            text: 'Percentile (%)',
            color: 'rgba(255, 255, 255, 0.8)'
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)',
            callback: (value) => value + '%'
          },
          min: 0,
          max: 100
        }
      }
    }
  }

  chart = new ChartJS(chartCanvas.value.getContext('2d')!, config)
}

function updateChart() {
  if (!chart || !props.data.length) return

  const sortedData = [...props.data].sort((a, b) => a.percentile - b.percentile)
  
  chart.data.labels = sortedData.map(d => d.symbol)
  chart.data.datasets[0].data = sortedData.map(d => d.percentile)
  
  chart.update('none')
}

onMounted(() => {
  createChart()
})

watch(() => props.data, () => {
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
.percentile-chart-container {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 300px;
}
</style>