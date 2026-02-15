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
    <div v-else-if="chartData.length === 0" class="no-data-state">
      <span>No volume data available</span>
    </div>
    <div v-else class="chart-wrapper">
      <canvas ref="chartCanvas"></canvas>
    </div>
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
let chart: Chart<'line', PointData[]> | null = null;

const chartData = computed<PointData[]>(() => {
  if (!props.labels?.length || !props.volumes?.length) return [];
  
  const minLength = Math.min(props.labels.length, props.volumes.length);
  const data: PointData[] = [];
  
  for (let i = 0; i < minLength; i++) {
    const label = props.labels[i];
    const volume = props.volumes[i];
    
    if (label instanceof Date && Number.isFinite(volume)) {
      data.push({
        x: label.getTime(),
        y: volume
      });
    }
  }
  
  return data.sort((a, b) => a.x - b.x);
});

const formatVolume = (volume: number): string => {
  if (volume >= 1e9) return `${(volume / 1e9).toFixed(1)}B`;
  if (volume >= 1e6) return `${(volume / 1e6).toFixed(1)}M`;
  if (volume >= 1e3) return `${(volume / 1e3).toFixed(1)}K`;
  return volume.toFixed(0);
};

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
        label: 'Volume',
        data: chartData.value,
        borderColor: '#FFA502',
        backgroundColor: 'rgba(255, 165, 2, 0.1)',
        borderWidth: 2,
        fill: true,
        tension: 0.2,
        pointRadius: 0,
        pointHoverRadius: 4,
      }]
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
          callbacks: {
            label: (context: TooltipItem<'line'>) => `Volume: ${formatVolume(context.parsed.y)}`
          }
        }
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
        y: {
          type: 'linear',
          position: 'left',
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: {
            color: '#ECF0F1',
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
  if (!chart || chartData.value.length === 0) {
    createChart();
    return;
  }
  
  chart.data.datasets[0].data = chartData.value;
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
</script>
