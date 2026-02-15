<template>
  <div class="returns-chart-container">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading returns & volatility...</span>
    </div>
    <div v-else-if="error" class="error-state">
      <span class="error-icon">⚠</span>
      <span>{{ error }}</span>
    </div>
    <div v-else-if="returnsData.length === 0 && volatilityData.length === 0" class="no-data-state">
      <span>No returns data available</span>
    </div>
    <div v-else class="chart-wrapper">
      <canvas ref="chartCanvas"></canvas>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed, nextTick } from 'vue'
import { Chart, registerables, type ChartConfiguration, type TooltipItem } from 'chart.js'
import 'chartjs-adapter-date-fns'

Chart.register(...registerables)

interface PointData { 
  x: number; 
  y: number; 
}

interface Props {
  labels: Date[]
  returns: number[]
  volatility: number[]
  loading?: boolean
  error?: string | null
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  error: null,
})

const chartCanvas = ref<HTMLCanvasElement | null>(null)
let chart: Chart<'line', PointData[]> | null = null

// Process and align data with proper validation
const returnsData = computed<PointData[]>(() => {
  if (!props.labels?.length || !props.returns?.length) return [];
  
  const minLength = Math.min(props.labels.length, props.returns.length);
  const data: PointData[] = [];
  
  for (let i = 0; i < minLength; i++) {
    const label = props.labels[i];
    const returnValue = props.returns[i];
    
    if (label instanceof Date && Number.isFinite(returnValue)) {
      data.push({
        x: label.getTime(),
        y: returnValue * 100 // Convert to percentage
      });
    }
  }
  
  return data.sort((a, b) => a.x - b.x);
});

const volatilityData = computed<PointData[]>(() => {
  if (!props.labels?.length || !props.volatility?.length) return [];
  
  const minLength = Math.min(props.labels.length, props.volatility.length);
  const data: PointData[] = [];
  
  for (let i = 0; i < minLength; i++) {
    const label = props.labels[i];
    const volValue = props.volatility[i];
    
    if (label instanceof Date && Number.isFinite(volValue)) {
      data.push({
        x: label.getTime(),
        y: volValue * 100 // Convert to percentage
      });
    }
  }
  
  return data.sort((a, b) => a.x - b.x);
});

function destroyChart() {
  if (chart) {
    chart.destroy();
    chart = null;
  }
}

async function createChart() {
  if (!chartCanvas.value || props.loading || props.error) {
    return;
  }

  if (returnsData.value.length === 0 && volatilityData.value.length === 0) {
    return;
  }

  await nextTick();
  
  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

  destroyChart();

  const config: ChartConfiguration<'line', PointData[]> = {
    type: 'line',
    data: {
      datasets: [
        {
          label: 'Daily Returns',
          data: returnsData.value,
          borderColor: '#00BF63',
          backgroundColor: 'rgba(0, 191, 99, 0.12)',
          borderWidth: 2,
          pointRadius: 0,
          pointHoverRadius: 4,
          tension: 0.2,
          yAxisID: 'returns',
          fill: false,
        },
        {
          label: 'Volatility',
          data: volatilityData.value,
          borderColor: '#FFA502',
          backgroundColor: 'rgba(255, 165, 2, 0.10)',
          borderWidth: 2,
          pointRadius: 0,
          pointHoverRadius: 4,
          tension: 0.2,
          yAxisID: 'volatility',
          fill: false,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      animation: { duration: 0 },
      interaction: { mode: 'index', intersect: false },
      plugins: {
        legend: { 
          display: true,
          labels: {
            color: '#ECF0F1',
            usePointStyle: true,
            padding: 15
          }
        },
        tooltip: {
          callbacks: {
            label: (ctx: TooltipItem<'line'>) => {
              const label = ctx.dataset.label ?? '';
              const val = Number(ctx.parsed.y).toFixed(2);
              return `${label}: ${val}%`;
            },
          },
        },
      },
      scales: {
        x: {
          type: 'time',
          time: { 
            unit: 'day', 
            displayFormats: { day: 'MMM dd' },
            tooltipFormat: 'MMM dd, yyyy'
          },
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: { color: '#ECF0F1' }
        },
        returns: {
          type: 'linear',
          position: 'left',
          title: { display: true, text: 'Returns (%)', color: '#ECF0F1' },
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: { 
            color: '#ECF0F1',
            callback: (v) => `${Number(v).toFixed(1)}%` 
          },
        },
        volatility: {
          type: 'linear',
          position: 'right',
          grid: { display: false },
          title: { display: true, text: 'Volatility (%)', color: '#ECF0F1' },
          ticks: { 
            color: '#ECF0F1',
            callback: (v) => `${Number(v).toFixed(1)}%` 
          },
        },
      },
    },
  };

  chart = new Chart(ctx, config);
}

function updateChart() {
  if (!chart) {
    createChart();
    return;
  }
  
  chart.data.datasets[0].data = returnsData.value;
  chart.data.datasets[1].data = volatilityData.value;
  chart.update('none');
}

onMounted(() => {
  if (!props.loading && !props.error && (returnsData.value.length > 0 || volatilityData.value.length > 0)) {
    createChart();
  }
});

onUnmounted(destroyChart);

watch(() => [returnsData.value, volatilityData.value], () => {
  if (!props.loading && !props.error && (returnsData.value.length > 0 || volatilityData.value.length > 0)) {
    updateChart();
  }
}, { deep: true });

watch(() => [props.loading, props.error], ([newLoading, newError]) => {
  if (!newLoading && !newError && (returnsData.value.length > 0 || volatilityData.value.length > 0)) {
    createChart();
  } else if (newLoading || newError) {
    destroyChart();
  }
});
</script>
