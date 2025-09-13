<template>
  <div class="signal-chart-container">
    <div class="chart-header">
      <h4 class="signal-title" :style="{ color: color }">{{ title }}</h4>
      <div class="signal-value" :class="strengthClass">
        {{ formatValue(currentValue) }}
      </div>
    </div>
    
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading signal...</span>
    </div>
    
    <div v-else-if="error" class="error-state">
      <span class="error-icon">⚠</span>
      <span>{{ error }}</span>
    </div>
    
    <div v-else class="chart-wrapper">
      <canvas ref="chartCanvas"></canvas>
    </div>
    
    <div v-if="!loading && !error" class="signal-stats">
      <div class="stat-item">
        <span class="stat-label">Current:</span>
        <span class="stat-value" :style="{ color }">{{ formatValue(currentValue) }}</span>
      </div>
      <div class="stat-item">
        <span class="stat-label">Strength:</span>
        <div class="strength-indicator">
          <div 
            class="strength-bar" 
            :style="{ 
              width: `${strengthPercentage}%`, 
              backgroundColor: color 
            }"
          ></div>
        </div>
      </div>
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
// Explicitly type the chart instance to resolve type conflicts
let chart: Chart<'line', PointData[]> | null = null;

const currentValue = computed(() => {
  return props.values.length > 0 ? props.values[props.values.length - 1] : 0;
});

const strengthClass = computed(() => {
  const abs = Math.abs(currentValue.value);
  if (abs > 2) return 'strong';
  if (abs > 1) return 'medium';
  return 'weak';
});

const strengthPercentage = computed(() => {
  return Math.min(Math.abs(currentValue.value) * 25, 100);
});

const chartData = computed<PointData[]>(() => {
  return props.values.map((value, index) => ({
    x: props.labels[index].getTime(), // Convert Date to numeric timestamp
    y: value,
  }));
});

function formatValue(value: number): string {
  return value.toFixed(2);
}

function createChart() {
  if (!chartCanvas.value || chart) return;
  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

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
            // Explicitly type the tooltip context parameter
            label: (context: TooltipItem<'line'>) => `${props.title}: ${context.parsed.y.toFixed(2)}`,
          },
        },
      },
      scales: {
        x: {
          type: 'time',
          time: { unit: 'day', displayFormats: { day: 'MMM dd' } },
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: { color: '#ECF0F1', display: false },
        },
        y: {
          grid: { color: 'rgba(236, 240, 241, 0.1)' },
          ticks: { color: '#ECF0F1', font: { size: 10 } },
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
  
  chart.data.datasets[0].data = chartData.value;
  chart.data.datasets[0].borderColor = props.color;
  chart.data.datasets[0].backgroundColor = `${props.color}20`;
  chart.options.plugins!.tooltip!.borderColor = props.color;
  chart.update('none');
}

onMounted(createChart);
onUnmounted(() => {
  chart?.destroy();
  chart = null;
});

watch(() => [props.labels, props.values, props.color], updateChart, { deep: true });
</script>
