<template>
  <div class="signal-chart-container">    
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading signal...</span>
    </div>
    
    <div v-else-if="error" class="error-state">
      <span class="error-icon">⚠</span>
      <span>{{ error }}</span>
    </div>
    
    <div v-else-if="chartData.length === 0" class="no-data-state">
      <span>No signal data available</span>
    </div>
    
    <div v-else class="chart-wrapper">
      <canvas ref="chartCanvas"></canvas>
    </div>
    
    <!-- The signal-stats section has been removed as it is now empty -->
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed, nextTick } from 'vue';
import { Chart, registerables, type ChartConfiguration, type TooltipItem } from 'chart.js';
import 'chartjs-adapter-date-fns';

Chart.register(...registerables);

interface PointData {
  x: number;
  y: number;
}

interface Props {
  title: string;
  color: string;
  labels: Date[];
  values: number[];
  loading?: boolean;
  error?: string | null;
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  error: null,
});

const chartCanvas = ref<HTMLCanvasElement | null>(null);
let chart: Chart<'line', PointData[]> | null = null;

const chartData = computed<PointData[]>(() => {
  if (!props.labels?.length || !props.values?.length) return [];
  
  const minLength = Math.min(props.labels.length, props.values.length);
  const data: PointData[] = [];
  
  for (let i = 0; i < minLength; i++) {
    const label = props.labels[i];
    const value = props.values[i];
    
    if (label instanceof Date && Number.isFinite(value)) {
      data.push({
        x: label.getTime(),
        y: value
      });
    }
  }
  
  return data.sort((a, b) => a.x - b.x);
});

const currentValue = computed(() => {
  return chartData.value.length > 0 ? chartData.value[chartData.value.length - 1].y : 0;
});

function destroyChart() {
  if (chart) {
    chart.destroy();
    chart = null;
  }
}

async function createChart() {
  if (!chartCanvas.value || props.loading || props.error || chartData.value.length === 0) {
    return;
  }

  await nextTick();
  
  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

  destroyChart();

  const config: ChartConfiguration<'line', PointData[]> = {
    type: 'line',
    data: {
      datasets: [{
        label: props.title,
        data: chartData.value,
        borderColor: props.color,
        backgroundColor: `${props.color}20`,
        borderWidth: 2,
        fill: true,
        tension: 0.2,
        pointRadius: 0,
        pointHoverRadius: 4,
      }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      animation: { duration: 0 },
      interaction: { mode: 'index', intersect: false },
      plugins: {
        legend: { display: false },
        tooltip: {
          mode: 'index',
          intersect: false,
          borderColor: props.color,
          callbacks: {
            label: (context: TooltipItem<'line'>) => `${props.title}: ${context.parsed.y.toFixed(2)}`,
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
          // Grid lines for the x-axis are now hidden
          grid: { 
            display: false,
          },
          ticks: { 
            color: '#ECF0F1', 
            display: false,
            maxTicksLimit: 100
          },
        },
        y: {
          // Grid lines for the y-axis are now hidden
          grid: { 
            display: false,
          },
          ticks: { 
            color: '#ECF0F1', 
            font: { size: 10 },
            maxTicksLimit: 8
          },
          title: {
            display: true,
            text: props.title,
            color: '#ECF0F1',
            font: {
              size: 10
            }
          }
        },
      },
    },
  };
  
  chart = new Chart(ctx, config);
}

function updateChart() {
  if (!chart || chartData.value.length === 0) {
    createChart();
    return;
  }
  
  chart.data.datasets[0].data = chartData.value;
  chart.data.datasets[0].borderColor = props.color;
  chart.data.datasets[0].backgroundColor = `${props.color}20`;
  if (chart.options.plugins?.tooltip) {
    chart.options.plugins.tooltip.borderColor = props.color;
  }
  chart.update('none');
}

onMounted(() => {
  if (!props.loading && !props.error && chartData.value.length > 0) {
    createChart();
  }
});

onUnmounted(destroyChart);

watch(() => chartData.value, (newData) => {
  if (newData.length > 0 && !props.loading && !props.error) {
    updateChart();
  }
}, { deep: true });

watch(() => [props.loading, props.error], ([newLoading, newError]) => {
  if (!newLoading && !newError && chartData.value.length > 0) {
    createChart();
  } else if (newLoading || newError) {
    destroyChart();
  }
});

watch(() => props.color, () => {
  if (chart && chartData.value.length > 0) {
    updateChart();
  }
});
</script>

