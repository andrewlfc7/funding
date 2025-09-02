<template>
  <div class="scatter-chart-container">
    <canvas ref="chartCanvas"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted } from 'vue'
import { ChartJS } from '@/utils/chartSetup'
import type { ChartConfiguration } from 'chart.js'

interface Props {
  data: Array<Record<string, any>>
  xField: string
  yField: string
  labelField?: string
  xLabel?: string
  yLabel?: string
  colorField?: string
  showLabels?: boolean
}

const props = defineProps<Props>()
const chartCanvas = ref<HTMLCanvasElement>()
let chart: ChartJS | null = null

function createChart() {
  if (!chartCanvas.value || !props.data || !Array.isArray(props.data) || props.data.length === 0) {
    console.warn('ScatterChart: No valid data provided')
    return
  }

  const config: ChartConfiguration<'scatter'> = {
    type: 'scatter',
    data: {
      datasets: [{
        label: 'Data',
        data: props.data.map(d => ({
          x: d[props.xField],
          y: d[props.yField]
        })),
        backgroundColor: 'rgba(99, 102, 241, 0.6)',
        borderColor: 'rgba(99, 102, 241, 1)',
        borderWidth: 1,
        pointRadius: 6,
        pointHoverRadius: 8
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
            label: (context) => {
              const index = context.dataIndex
              const item = props.data[index]
              const label = props.labelField ? item[props.labelField] : ''
              const x = context.parsed.x.toFixed(2)
              const y = context.parsed.y.toFixed(2)
              return label ? `${label}: (${x}, ${y})` : `(${x}, ${y})`
            }
          }
        }
      },
      scales: {
        x: {
          title: {
            display: true,
            text: props.xLabel || props.xField,
            color: 'rgba(255, 255, 255, 0.8)'
          },
          grid: {
            color: 'rgba(255, 255, 255, 0.05)'
          },
          ticks: {
            color: 'rgba(255, 255, 255, 0.6)'
          }
        },
        y: {
          title: {
            display: true,
            text: props.yLabel || props.yField,
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

  // If we want to show labels, we need to add annotations
  if (props.showLabels && props.labelField) {
    // Add text annotations for extreme points
    const annotations: any = {}; // Added semicolon here
    
    props.data.forEach((item, index) => {
      // Only label extreme points (customize this logic as needed)
      const absX = Math.abs(item[props.xField])
      if (absX > 1.5 && props.labelField) { // Added check for labelField
        annotations[`label${index}`] = {
          type: 'label',
          xValue: item[props.xField],
          yValue: item[props.yField],
          content: item[props.labelField],
          color: 'rgba(255, 255, 255, 0.8)',
          font: {
            size: 10,
            weight: 'bold'
          },
          position: item[props.yField] > 0 ? 'end' : 'start'
        }
      }
    })
    
    // Add annotation plugin configuration
    ;(config.options as any).plugins.annotation = {
      annotations
    }
  }

  chart = new ChartJS(chartCanvas.value.getContext('2d')!, config)
}

function updateChart() {
  if (!chart || !props.data || !Array.isArray(props.data) || props.data.length === 0) {
    console.warn('ScatterChart: Cannot update - no valid data')
    return
  }

  chart.data.datasets[0].data = props.data.map(d => ({
    x: d[props.xField],
    y: d[props.yField]
  }))
  
  chart.update('none')
}

onMounted(() => {
  createChart()
})

watch(() => [props.data, props.xField, props.yField], () => {
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
.scatter-chart-container {
  position: relative;
  width: 100%;
  height: 100%;

}

/* Ensure canvas fills container */
.scatter-chart-container canvas {
  max-height: 100%;
}
</style>