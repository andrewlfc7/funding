<template>
  <div class="returns-chart-container">
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading returns data...</span>
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
import { Chart, registerables, type ChartConfiguration, type ScriptableContext, type TooltipItem, type LegendItem } from 'chart.js';
import 'chartjs-adapter-date-fns';

Chart.register(...registerables);

// Define a consistent data point structure for Chart.js
interface PointData {
  x: number;
  y: number;
}

interface Props {
  labels: Date[];
  returns: number[];
  volatility: number[];
  loading?: boolean;
  error?: string | null;
}

const props = withDefaults(defineProps<Props>(), {
  loading: false,
  error: null,
});

const chartCanvas = ref<HTMLCanvasElement | null>(null);
// Explicitly type the chart instance for mixed chart types
let chart: Chart<'line' | 'bar', PointData[]> | null = null;

const returnsData = computed<PointData[]>(() => {
  return props.returns.map((ret, index) => ({
    x: props.labels[index].getTime(), // Convert Date to numeric timestamp
    y: ret * 100,
  }));
});

const volatilityData = computed<PointData[]>(() => {
  return props.volatility.map((vol, index) => ({
    x: props.labels[index].getTime(), // Convert Date to numeric timestamp
    y: vol * 100,
  }));
});

const volatilityBands = computed(() => {
  const sortedVols = [...props.volatility].sort((a, b) => a - b);
  const p25 = (sortedVols[Math.floor(sortedVols.length * 0.25)] || 0) * 100;
  const p75 = (sortedVols[Math.floor(sortedVols.length * 0.75)] || 0) * 100;
  
  return {
    p25: props.labels.map(label => ({ x: label.getTime(), y: p25 })),
    p75: props.labels.map(label => ({ x: label.getTime(), y: p75 })),
  };
});

function createChart() {
  if (!chartCanvas.value || chart) return;
  const ctx = chartCanvas.value.getContext('2d');
  if (!ctx) return;

  const config: ChartConfiguration<'line' | 'bar', PointData[]> = {
    type: 'line',
    data: {
      datasets: [
        {
          label: 'Daily Returns',
          data: returnsData.value,
          type: 'bar',
          // Explicitly type context for scriptable options
          backgroundColor: (context: ScriptableContext<'bar'>) => 
            (context.parsed?.y ?? 0) > 0 ? 'rgba(0, 191, 99, 0.6)' : 'rgba(255, 71, 87, 0.6)',
          borderColor: (context: ScriptableContext<'bar'>) => 
            (context.parsed?.y ?? 0) > 0 ? '#00BF63' : '#FF4757',
          borderWidth: 1,
          yAxisID: 'returns',
        },
        {
          label: 'Rolling Volatility',
          data: volatilityData.value,
          borderColor: '#FFA502',
          backgroundColor: 'rgba(255, 165, 2, 0.1)',
          borderWidth: 2,
          fill: false,
          tension: 0.2,
          pointRadius: 0,
          yAxisID: 'volatility',
        },
        {
          label: 'Vol 75th Percentile',
          data: volatilityBands.value.p75,
          borderColor: 'rgba(255, 165, 2, 0.5)',
          borderWidth: 1,
          borderDash: [5, 5],
          pointRadius: 0,
          yAxisID: 'volatility',
        },
        {
          label: 'Vol 25th Percentile',
          data: volatilityBands.value.p25,
          borderColor: 'rgba(255, 165, 2, 0.3)',
          borderWidth: 1,
          borderDash: [5, 5],
          pointRadius: 0,
          yAxisID: 'volatility',
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: 'index', intersect: false },
      plugins: {
        legend: {
          labels: {
            // Explicitly type legend item
            filter: (item: LegendItem) => !item.text.includes('Percentile'),
          },
        },
        tooltip: {
          callbacks: {
            // Explicitly type tooltip item
            label: (context: TooltipItem<'line' | 'bar'>) => {
              const label = context.dataset.label || '';
              const value = context.parsed.y.toFixed(2);
              if (label === 'Daily Returns') return `Returns: ${value}%`;
              if (label === 'Rolling Volatility') return `Volatility: ${value}%`;
              return `${label}: ${value}%`;
            },
          },
        },
      },
      scales: {
        x: {
          type: 'time',
          time: { unit: 'day', displayFormats: { day: 'MMM dd' } },
        },
        returns: {
          type: 'linear',
          position: 'left',
          title: { display: true, text: 'Daily Returns (%)' },
          ticks: { callback: (value) => `${Number(value).toFixed(1)}%` },
        },
        volatility: {
          type: 'linear',
          position: 'right',
          grid: { display: false },
          title: { display: true, text: 'Volatility (%)' },
          ticks: { callback: (value) => `${Number(value).toFixed(1)}%` },
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
  chart.data.datasets[2].data = volatilityBands.value.p75;
  chart.data.datasets[3].data = volatilityBands.value.p25;
  chart.update('none');
}

onMounted(createChart);
onUnmounted(() => {
  chart?.destroy();
  chart = null;
});

watch(() => [props.labels, props.returns, props.volatility], updateChart, { deep: true });
</script>
