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
    <div v-else-if="!hasValidData" class="no-data-state">
      <span>No price data available</span>
    </div>
    <div v-else class="chart-wrapper">
      <canvas ref="chartCanvas"></canvas>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed, nextTick } from 'vue';
import { Chart, registerables, type ChartConfiguration } from 'chart.js';
import 'chartjs-adapter-date-fns';

Chart.register(...registerables);

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
let chart: Chart<'line', PointData[]> | null = null;

// Add validation for data
const hasValidData = computed(() => {
  return Array.isArray(props.labels) && 
         Array.isArray(props.prices) && 
         props.labels.length > 0 && 
         props.prices.length > 0 &&
         props.labels.length === props.prices.length;
});

const chartData = computed<PointData[]>(() => {
  if (!hasValidData.value) return [];
  
  const data: PointData[] = [];
  
  for (let i = 0; i < props.labels.length; i++) {
    const label = props.labels[i];
    const price = props.prices[i];
    
    // More robust validation
    if (label instanceof Date && 
        !isNaN(label.getTime()) && 
        Number.isFinite(price) && 
        price > 0) {
      data.push({
        x: label.getTime(),
        y: price
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
  if (!chartCanvas.value || props.loading || props.error || !hasValidData.value) {
    return;
  }

  // Wait for DOM updates
  await nextTick();
  await new Promise(resolve => setTimeout(resolve, 50));
  
  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

  destroyChart();

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
            label: (context) => `Price: $${context.parsed.y.toLocaleString()}`
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
  
  if (!hasValidData.value) {
    destroyChart();
    return;
  }
  
  chart.data.datasets[0].data = chartData.value;
  chart.update('none');
}

// Watch for prop changes with debouncing
let updateTimeout: number | null = null;

const debouncedUpdate = () => {
  if (updateTimeout) clearTimeout(updateTimeout);
  updateTimeout = setTimeout(() => {
    if (!props.loading && !props.error && hasValidData.value) {
      updateChart();
    } else if (props.loading || props.error || !hasValidData.value) {
      destroyChart();
    }
  }, 100);
};

onMounted(() => {
  debouncedUpdate();
});

onUnmounted(() => {
  if (updateTimeout) clearTimeout(updateTimeout);
  destroyChart();
});

// Watch for data changes
watch(() => [props.labels, props.prices], debouncedUpdate, { deep: true });
watch(() => [props.loading, props.error], debouncedUpdate);
</script>