<template>
  <div class="chart-container">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading price...</span>
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
import { ref, onMounted, onUnmounted, watch, computed } from 'vue';
import { Chart, registerables, type ChartConfiguration, type ChartTypeRegistry } from 'chart.js';
import 'chartjs-adapter-date-fns';

Chart.register(...registerables);

// Define the structure for individual data points
interface PointData {
  x: number;
  y: number;
}

interface Props {
  labels: Date[];
  prices: number[];
  loading?: boolean;
  error?: string | null;
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  error: null,
});

const chartCanvas = ref<HTMLCanvasElement | null>(null);
// Explicitly type the chart instance to prevent type conflicts
let chart: Chart<'line', PointData[]> | null = null;

const chartData = computed<PointData[]>(() => {
  return props.prices.map((price, index) => ({
    // Convert Date to milliseconds for Chart.js compatibility
    x: props.labels[index].getTime(), 
    y: price
  }));
});

function createChart() {
  if (!chartCanvas.value || chart) return;
  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

  const config: ChartConfiguration<'line', PointData[]> = {
    type: 'line',
    data: {
      datasets: [{
        label: 'Price',
        data: chartData.value,
        borderColor: '#00D4FF',
        backgroundColor: 'rgba(0, 212, 255, 0.1)',
        borderWidth: 2,
        fill: true,
        tension: 0.1,
        pointRadius: 0,
      }]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: 'index', intersect: false },
      plugins: {
        legend: { display: false },
        tooltip: {
          mode: 'index',
          intersect: false,
          callbacks: {
            label: (context) => `Price: $${context.parsed.y.toLocaleString()}`
          }
        }
      },
      scales: {
        x: {
          type: 'time',
          time: { unit: 'day', displayFormats: { day: 'MMM dd' } },
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: { color: '#ECF0F1' }
        },
        y: {
          type: 'linear',
          position: 'left',
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: {
            color: '#ECF0F1',
            callback: (value) => `$${Number(value).toLocaleString()}`
          },
          title: { display: true, text: 'Price ($)', color: '#ECF0F1' }
        }
      }
    }
  };
  
  chart = new Chart(ctx, config);
}

function updateChart() {
  if (!chart) {
    createChart();
    return;
  }
  chart.data.datasets[0].data = chartData.value;
  chart.update('none');
}

onMounted(createChart);
onUnmounted(() => {
  chart?.destroy();
  chart = null;
});

watch(() => [props.labels, props.prices], updateChart, { deep: true });
</script>
