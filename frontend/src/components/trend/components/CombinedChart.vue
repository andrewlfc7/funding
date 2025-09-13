<template>
  <div class="combined-chart-container">
    <div class="chart-header">
      <h4>Combined Signals</h4>
      <div class="signal-toggles">
        <button
          v-for="signal in availableSignals"
          :key="signal.id"
          @click="toggleSignal(signal.id)"
          :class="['signal-toggle', { active: activeSignals[signal.id] }]"
          :style="{ 
            borderColor: activeSignals[signal.id] ? signal.color : '#34495E',
            color: activeSignals[signal.id] ? signal.color : '#BDC3C7'
          }"
        >
          {{ signal.name }}
        </button>
      </div>
    </div>
    
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading signals...</span>
    </div>
    
    <div v-else-if="error" class="error-state">
      <span class="error-icon">⚠</span>
      <span>{{ error }}</span>
    </div>
    
    <div v-else class="chart-wrapper">
      <canvas ref="chartCanvas"></canvas>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import { Chart, registerables, type ChartTypeRegistry, type ChartConfiguration, type Point } from 'chart.js'
import 'chartjs-adapter-date-fns'

Chart.register(...registerables)

// Define a more specific chart type to avoid casting issues
type LineChartWithTime = Chart<'line', (number | Point | null)[], unknown>;

interface Signal {
  id: string
  name: string
  color: string
}

interface Props {
  labels: Date[]
  dataById: Record<string, number[]>
  signals?: Signal[]
  activeSignals?: Record<string, boolean>
  loading?: boolean
  error?: string | null
}

const props = withDefaults(defineProps<Props>(), {
  signals: () => [
    { id: 'momentum', name: 'Momentum', color: '#00BF63' },
    { id: 'ewmac', name: 'EWMAC', color: '#00D4FF' },
    { id: 'breakout', name: 'Breakout', color: '#FF6B6B' },
    { id: 'composite', name: 'Composite', color: '#FFA502' }
  ],
  activeSignals: () => ({
    momentum: true,
    ewmac: true,
    breakout: true,
    composite: true
  }),
  loading: false,
  error: null
})

const emit = defineEmits<{
  'update:activeSignals': [signals: Record<string, boolean>]
}>()

const chartCanvas = ref<HTMLCanvasElement | null>(null)
let chart: LineChartWithTime | null = null

const availableSignals = computed(() => props.signals!)
const localActiveSignals = ref({ ...props.activeSignals! })

const chartData = computed(() => {
  return availableSignals.value
    .filter(signal => localActiveSignals.value[signal.id])
    .map(signal => ({
      label: signal.name,
      // FIX: Convert Date to number for TypeScript compatibility
      // The date adapter will still interpret this as a timestamp.
      data: (props.dataById[signal.id] || []).map((value, index) => ({
        x: props.labels[index].getTime(), 
        y: value
      })),
      borderColor: signal.color,
      backgroundColor: `${signal.color}20`,
      borderWidth: 2,
      fill: false,
      tension: 0.2,
      pointRadius: 0,
      pointHoverRadius: 4
    }))
})

function toggleSignal(signalId: string) {
  localActiveSignals.value[signalId] = !localActiveSignals.value[signalId]
  emit('update:activeSignals', { ...localActiveSignals.value })
  updateChart()
}

function createChart() {
  if (!chartCanvas.value || chart) return
  
  const ctx = chartCanvas.value.getContext('2d')
  if (!ctx) return

  // Use the specific chart type here
  chart = new Chart(ctx, {
    type: 'line',
    data: {
      datasets: chartData.value,
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      animation: {
        duration: 0
      },
      interaction: {
        mode: 'index',
        intersect: false
      },
      plugins: {
        legend: {
          display: false
        },
        tooltip: {
          mode: 'index',
          intersect: false,
          backgroundColor: 'rgba(44, 62, 80, 0.9)',
          titleColor: '#ECF0F1',
          bodyColor: '#ECF0F1',
          borderColor: '#00D4FF',
          borderWidth: 1,
          callbacks: {
            label: function(context) {
              return `${context.dataset.label}: ${context.parsed.y.toFixed(2)}`
            }
          }
        }
      },
      scales: {
        x: {
          type: 'time',
          time: {
            unit: 'day',
            displayFormats: {
              day: 'MMM dd'
            }
          },
          grid: {
            color: 'rgba(236, 240, 241, 0.1)'
          },
          ticks: {
            color: '#ECF0F1',
            maxRotation: 0
          }
        },
        y: {
          grid: {
            color: 'rgba(236, 240, 241, 0.1)'
          },
          ticks: {
            color: '#ECF0F1'
          },
          title: {
            display: true,
            text: 'Signal Strength (Z-Score)',
            color: '#ECF0F1'
          }
        }
      }
    }
  }) as LineChartWithTime;
}

function updateChart() {
  if (!chart) {
    createChart()
    return
  }
  
  // The types now match, so no error here.
  chart.data.datasets = chartData.value;
  chart.update('none')
}

watch(() => props.activeSignals, (newSignals) => {
  if (newSignals) {
    localActiveSignals.value = { ...newSignals }
    updateChart()
  }
}, { deep: true })

watch(() => [props.labels, props.dataById], () => {
  updateChart()
}, { deep: true })

</script>
