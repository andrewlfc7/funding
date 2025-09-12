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


function generateColorPalette(count: number): string[] {
  const colors: string[] = [];
  const saturation = 70; // A good balance for vibrant colors
  const lightness = 20; // Ensures colors are not too dark or washed out


  const goldenRatioConjugate = 0.61803398875;
  let hue = Math.random(); // Start with a random hue

  for (let i = 0; i < count; i++) {
    hue = (hue + goldenRatioConjugate) % 1; // Increment hue using the golden ratio
    const finalHue = Math.floor(hue * 360);
    colors.push(`hsl(${finalHue}, ${saturation}%, ${lightness}%)`);
  }

  return colors;
}

function hslToRgba(hsl: string, alpha: number = 1): string {
  // Extract HSL values
  const match = hsl.match(/hsl\((\d+),\s*(\d+)%,\s*(\d+)%\)/);
  if (!match) return hsl;

  const h = parseInt(match[1]) / 360;
  const s = parseInt(match[2]) / 100;
  const l = parseInt(match[3]) / 100;

  // Convert to RGB
  const hue2rgb = (p: number, q: number, t: number) => {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1/6) return p + (q - p) * 6 * t;
    if (t < 1/2) return q;
    if (t < 2/3) return p + (q - p) * (2/3 - t) * 6;
    return p;
  };

  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const r = Math.round(hue2rgb(p, q, h + 1/3) * 255);
  const g = Math.round(hue2rgb(p, q, h) * 255);
  const b = Math.round(hue2rgb(p, q, h - 1/3) * 255);

  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}


function createChart() {
  if (!chartCanvas.value || !props.data || !Array.isArray(props.data) || props.data.length === 0) {
    console.warn('ScatterChart: No valid data provided')
    return
  }

  // Get unique symbols for color mapping
  const uniqueSymbols = [...new Set(props.data.map(d => d[props.labelField || 'symbol'] || 'unknown'))]
  const colorPalette = generateColorPalette(uniqueSymbols.length)
  const colorMap = new Map<string, { bg: string, border: string }>()
  
  uniqueSymbols.forEach((symbol, index) => {
    const baseColor = colorPalette[index]
    colorMap.set(symbol, {
      bg: hslToRgba(baseColor, 0.7),
      border: hslToRgba(baseColor, 1)
    })
  })

  const config: ChartConfiguration<'scatter'> = {
    type: 'scatter',
    data: {
      datasets: [{
        label: 'Data',
        data: props.data.map((d, index) => ({
          x: d[props.xField],
          y: d[props.yField],
          symbol: d[props.labelField || 'symbol'] || 'unknown'
        })),
        backgroundColor: (context) => {
          const dataPoint = context.parsed
          const symbol = (context.raw as any)?.symbol || 'unknown'
          return colorMap.get(symbol)?.bg || 'rgba(99, 102, 241, 0.6)'
        },
        borderColor: (context) => {
          const dataPoint = context.parsed
          const symbol = (context.raw as any)?.symbol || 'unknown'
          return colorMap.get(symbol)?.border || 'rgba(99, 102, 241, 1)'
        },
        borderWidth: 2,
        pointRadius: 6,
        pointHoverRadius: 8,
        pointHoverBorderWidth: 3
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
          },
          backgroundColor: 'rgba(0, 0, 0, 0.8)',
          titleColor: 'white',
          bodyColor: 'white',
          borderColor: 'rgba(255, 255, 255, 0.2)',
          borderWidth: 1
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
    const annotations: any = {}
    
    props.data.forEach((item, index) => {
      // Only label extreme points (customize this logic as needed)
      const absX = Math.abs(item[props.xField])
      if (absX > 1.5 && props.labelField) {
        const symbol = item[props.labelField]
        const colors = colorMap.get(symbol)
        
        annotations[`label${index}`] = {
          type: 'label',
          xValue: item[props.xField],
          yValue: item[props.yField],
          content: symbol,
          color: colors?.border || 'rgba(255, 255, 255, 0.8)',
          backgroundColor: 'rgba(0, 0, 0, 0.7)',
          borderColor: colors?.border || 'rgba(255, 255, 255, 0.3)',
          borderWidth: 1,
          borderRadius: 4,
          padding: 4,
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

  // Recreate the chart to ensure colors are properly updated
  chart.destroy()
  createChart()
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