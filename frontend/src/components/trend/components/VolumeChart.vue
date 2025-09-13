<template>
  <div class="chart-container">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading volume...</span>
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
import { Chart, registerables, type ChartConfiguration, type TooltipItem } from 'chart.js';
import 'chartjs-adapter-date-fns';

Chart.register(...registerables);

// Define a consistent data point structure for Chart.js
interface PointData {
  x: number;
  y: number;
}

interface Props {
  labels: Date[];
  volumes: number[];
  loading?: boolean;
  error?: string | null;
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  error: null,
});

const chartCanvas = ref<HTMLCanvasElement | null>(null);
// Explicitly type the chart instance
let chart: Chart<'line', PointData[]> | null = null;

const chartData = computed<PointData[]>(() => {
  return props.volumes.map((volume, index) => ({
    x: props.labels[index].getTime(), // Convert Date to numeric timestamp
    y: volume,
  }));
});

const formatVolume = (volume: number): string => {
  if (volume >= 1e9) return `${(volume / 1e9).toFixed(1)}B`;
  if (volume >= 1e6) return `${(volume / 1e6).toFixed(1)}M`;
  if (volume >= 1e3) return `${(volume / 1e3).toFixed(1)}K`;
  return volume.toString();
};

function createChart() {
  if (!chartCanvas.value || chart) return;
  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

  const config: ChartConfiguration<'line', PointData[]> = {
    type: 'line', // Changed from 'bar' to 'line'
    data: {
      datasets: [{
        label: 'Volume',
        data: chartData.value,
        borderColor: '#FFA502',
        backgroundColor: 'rgba(255, 165, 2, 0.1)', // More suitable for line chart
        borderWidth: 2,
        fill: true, // Fill under the line
        tension: 0.2,
        pointRadius: 0,
        pointHoverRadius: 4,
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
            label: (context: TooltipItem<'line'>) => `Volume: ${formatVolume(context.parsed.y)}`
          }
        }
      },
      scales: {
        x: {
          type: 'time',
          time: { unit: 'day', displayFormats: { day: 'MMM dd' } },
          grid: { display: false },
          ticks: { color: '#ECF0F1', font: { size: 10 } }
        },
        y: {
          type: 'linear',
          position: 'left',
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: {
            color: '#ECF0F1',
            font: { size: 10 },
            callback: (value) => formatVolume(Number(value))
          },
          title: { display: true, text: 'Volume', color: '#ECF0F1' }
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

watch(() => [props.labels, props.volumes], updateChart, { deep: true });
</script>
