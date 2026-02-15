<template>
  <div class="market-signals-dashboard">
    <div class="dashboard-controls">
      <div class="control-group">
        <label>Asset:</label>
        <select v-model="selectedCoin" @change="handleCoinChange" class="control-select">
          <option v-for="symbol in availableSymbols" :key="symbol" :value="symbol">
            {{ symbol }}
          </option>
        </select>
      </div>
      
      <div class="control-group">
        <label>Exchange:</label>
        <select v-model="selectedExchange" @change="handleExchangeChange" class="control-select">
          <option value="binance">Binance</option>
        </select>
      </div>
      
      <div class="control-group">
        <label>Period:</label>
        <select v-model="selectedPeriod" @change="handlePeriodChange" class="control-select">
          <option value="30d">30 Days</option>
          <option value="60d">60 Days</option>
          <option value="90d">90 Days</option>
        </select>
      </div>

      <div class="debug-info" v-if="showDebug">
        <small>
          Symbols: {{ availableSymbols.length }} | 
          Prices: {{ priceValues.length }} | 
          Signals: {{ Object.keys(signalsBySymbol).length }} |
          Loading: {{ loading }}
        </small>
      </div>
    </div>

    <div class="signals-grid">
      <div class="grid-item price-panel">
        <h3>Price - {{ selectedCoin }}</h3>
        <div class="panel-content">
          <PriceChart
            :labels="priceLabels"
            :prices="priceValues"
            :loading="loading"
            :error="error"
            :key="`price-${dataVersion}`"
          />
        </div>
      </div>

      <div class="grid-item volume-panel">
        <h3>Volume</h3>
        <div class="panel-content">
          <VolumeChart
            :labels="volumeLabels"
            :volumes="volumeValues"
            :loading="loading"
            :error="error"
            :key="`volume-${dataVersion}`"
          />
        </div>
      </div>

      <div class="grid-item returns-vol-panel">
        <h3>Returns & Volatility</h3>
        <div class="panel-content">
          <ReturnsChart
            :labels="returnsLabels"
            :returns="returnsValues"
            :volatility="volatilityValues"
            :loading="loading"
            :error="error"
            :key="`returns-${dataVersion}`"
          />
          <div class="vol-percentiles">
            <div class="percentile-item">
              <span class="label">Vol 25th:</span>
              <span class="value">{{ formatPercent(vol25th) }}</span>
            </div>
            <div class="percentile-item">
              <span class="label">Vol 75th:</span>
              <span class="value">{{ formatPercent(vol75th) }}</span>
            </div>
            <div class="percentile-item">
              <span class="label">Current:</span>
              <span class="value current-vol">{{ formatPercent(currentVol) }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="grid-item ewmac-panel">
        <h3>EWMAC Signal</h3>
        <div class="panel-content">
          <SignalChart
            title="EWMAC"
            color="#00D4FF"
            :labels="signalLabels"
            :values="ewmacValues"
            :loading="loading"
            :error="error"
            :key="`ewmac-${dataVersion}`"
          />
        </div>
      </div>

      <div class="grid-item breakout-panel">
        <h3>Breakout Signal</h3>
        <div class="panel-content">
          <SignalChart
            title="Breakout"
            color="#FF6B6B"
            :labels="signalLabels"
            :values="breakoutValues"
            :loading="loading"
            :error="error"
            :key="`breakout-${dataVersion}`"
          />
        </div>
      </div>

      <div class="grid-item momentum-panel">
        <h3>Momentum Signal</h3>
        <div class="panel-content">
          <SignalChart
            title="Momentum"
            color="#00BF63"
            :labels="signalLabels"
            :values="momentumValues"
            :loading="loading"
            :error="error"
            :key="`momentum-${dataVersion}`"
          />
        </div>
      </div>

      <div class="grid-item trend-panel">
        <h3>Trend Factor</h3>
        <div class="panel-content">
          <SignalChart
            title="Trend"
            color="#7C5CFF"
            :labels="signalLabels"
            :values="trendValues"
            :loading="loading"
            :error="error"
            :key="`trend-${dataVersion}`"
          />
        </div>
      </div>

      <div class="grid-item combined-panel">
        <h3>Combined Signals</h3>
        <div class="panel-content">
          <CombinedChart
            :labels="signalLabels"
            :dataById="signalDataById"
            :activeSignals="activeSignals"
            @update:activeSignals="activeSignals = $event"
            :loading="loading"
            :error="error"
            :key="`combined-${dataVersion}`"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, inject, type Ref, watch, onMounted } from 'vue'
import PriceChart from '../components/PriceChart.vue'
import VolumeChart from '../components/VolumeChart.vue'
import ReturnsChart from '../components/ReturnsChart.vue'
import SignalChart from '../components/SignalChart.vue'
import CombinedChart from '../components/CombinedChart.vue'
import { fetchMetaData } from '@/api/meta'

/* ---------- Interfaces ---------- */
interface TrendSignalPoint {
  ts: number
  symbol: string
  trend: number
  momentum: number
  ewmac: number
  breakout: number
  composite: number
}

interface TimeSeriesPoint {
  date: string
  value: number
}

interface VolumePoint {
  date: string
  volume_ewma: number
  dollar_volume_ewma: number
}

interface Controls {
  selectedCoin: Ref<string>
  selectedExchange: Ref<string>
  selectedPeriod: Ref<string>
  reloadAll: () => void
}

interface MarketDataProvider {
  priceSeries: Ref<TimeSeriesPoint[]>
  returnsSeries: Ref<TimeSeriesPoint[]>
  volSeries: Ref<TimeSeriesPoint[]>
  volumeSeries: Ref<VolumePoint[]>
  loading: Ref<boolean>
  error: Ref<string | null>
}

interface SignalDataProvider {
  series: Ref<Record<string, TrendSignalPoint[]>>
  symbols: Ref<string[]>
  loading: Ref<boolean>
  error: Ref<string | null>
}

type ActiveSignals = Record<string, boolean>

/* ---------- Injections ---------- */
const controls = inject<Controls>('controls')!
const { selectedCoin, selectedExchange, selectedPeriod, reloadAll } = controls

const marketData = inject<MarketDataProvider>('marketData')!
const signalData = inject<SignalDataProvider>('signalData')!

// Create reactive references that properly track changes
const priceSeries = computed(() => marketData.priceSeries.value || [])
const returnsSeries = computed(() => marketData.returnsSeries.value || [])
const volSeries = computed(() => marketData.volSeries.value || [])
const volumeSeries = computed(() => marketData.volumeSeries.value || [])
const loading = computed(() => marketData.loading.value || signalData.loading.value)
const error = computed(() => marketData.error.value || signalData.error.value)

const showDebug = ref(false)

/* ---------- Local State ---------- */
const activeSignals = ref<ActiveSignals>({
  momentum: true,
  ewmac: true,
  breakout: true,
  composite: true,
  trend: true,
})
const metaCoins = ref<string[]>([])
const lastLoadedParams = ref<{ coin: string; exchange: string; period: string } | null>(null)

// Add a data version tracker to force chart re-renders
const dataVersion = ref(0)

/* ---------- Lifecycle Hooks ---------- */
onMounted(async () => {
  try {
    const meta = await fetchMetaData('spot', 'USDT');
    metaCoins.value = meta.coins.sort();
  } catch (e) {
    console.error("Failed to fetch metadata for dropdown", e);
  }
});

/* ---------- Data Processing ---------- */
const MAX_DATA_POINTS = 90

const getLatestData = <T>(data: T[], maxPoints: number = MAX_DATA_POINTS): T[] => {
  if (!data || !data.length) return [];
  return data.slice(-maxPoints);
}

// Helper function to safely parse dates
const parseDate = (dateStr: string): Date => {
  const date = new Date(dateStr);
  return isNaN(date.getTime()) ? new Date() : date;
}

// Helper function to convert timestamp to Date
const timestampToDate = (ts: number): Date => {
  return new Date(ts);
}

/* ---------- Computed Properties ---------- */
const availableSymbols = computed(() => {
  const allSymbols = metaCoins.value;
  if (allSymbols.length === 0) return ['BTC'];
  
  const majorCoins = ['BTC', 'ETH', 'BNB', 'ADA', 'XRP', 'SOL', 'DOT', 'AVAX', 'MATIC', 'LTC'];
  const availableSet = new Set(allSymbols);
  
  const result = majorCoins.filter(coin => availableSet.has(coin));
  const otherCoins = allSymbols
    .filter(coin => !majorCoins.includes(coin))
    .sort();
  
  return [...result, ...otherCoins];
})

const signalsBySymbol = computed(() => signalData.series.value || {})

// Improved chart data processing with better error handling
const priceChartData = computed(() => {
  const data = getLatestData(priceSeries.value);
  if (!data.length) return { labels: [], values: [] };
  
  return {
    labels: data.map((p: TimeSeriesPoint) => parseDate(p.date)),
    values: data.map((p: TimeSeriesPoint) => Number(p.value) || 0)
  };
});

const returnsChartData = computed(() => {
  const data = getLatestData(returnsSeries.value);
  if (!data.length) return { labels: [], values: [] };
  
  return {
    labels: data.map((r: TimeSeriesPoint) => parseDate(r.date)),
    values: data.map((r: TimeSeriesPoint) => Number(r.value) || 0)
  };
});

const volChartData = computed(() => {
  const data = getLatestData(volSeries.value);
  if (!data.length) return { labels: [], values: [] };
  
  return {
    labels: data.map((v: TimeSeriesPoint) => parseDate(v.date)),
    values: data.map((v: TimeSeriesPoint) => Number(v.value) || 0)
  };
});

const volumeChartData = computed(() => {
  const data = getLatestData(volumeSeries.value);
  if (!data.length) return { labels: [], values: [] };
  
  return {
    labels: data.map((v: VolumePoint) => parseDate(v.date)),
    values: data.map((v: VolumePoint) => Number(v.dollar_volume_ewma) || 0)
  };
});

const priceLabels = computed(() => priceChartData.value.labels)
const priceValues = computed(() => priceChartData.value.values)
const returnsLabels = computed(() => returnsChartData.value.labels)
const returnsValues = computed(() => returnsChartData.value.values)
const volatilityValues = computed(() => volChartData.value.values)
const volumeLabels = computed(() => volumeChartData.value.labels)
const volumeValues = computed(() => volumeChartData.value.values)

const coinSignals = computed(() => {
  const seriesData = signalsBySymbol.value[selectedCoin.value];
  if (!seriesData) return [];
  const sortedSignals = [...seriesData].sort((a, b) => a.ts - b.ts);
  return getLatestData(sortedSignals);
});

const signalLabels = computed(() => {
  if (!coinSignals.value.length) return [];
  return coinSignals.value.map(s => timestampToDate(s.ts));
});

const signalDataById = computed((): Record<string, number[]> => {
  const signals = coinSignals.value;
  const result: Record<string, number[]> = { momentum: [], ewmac: [], breakout: [], composite: [], trend: [] };
  if (!signals || signals.length === 0) return result;
  
  for (const s of signals) {
    result.momentum.push(Number(s.momentum) || 0);
    result.ewmac.push(Number(s.ewmac) || 0);
    result.breakout.push(Number(s.breakout) || 0);
    result.composite.push(Number(s.composite) || 0);
    result.trend.push(Number(s.trend) || 0);
  }
  return result;
});

const ewmacValues = computed(() => signalDataById.value.ewmac)
const breakoutValues = computed(() => signalDataById.value.breakout)
const momentumValues = computed(() => signalDataById.value.momentum)
const trendValues = computed(() => signalDataById.value.trend)

const last = (arr: number[]) => (arr?.length ? arr[arr.length - 1] : 0);

const currentVol = computed(() => last(volatilityValues.value));

const vol25th = computed(() => {
  const sorted = [...volatilityValues.value].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length * 0.25)] ?? 0;
});

const vol75th = computed(() => {
  const sorted = [...volatilityValues.value].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length * 0.75)] ?? 0;
});

/* ---------- Event Handlers ---------- */
function shouldReload(): boolean {
  const current = { coin: selectedCoin.value, exchange: selectedExchange.value, period: selectedPeriod.value };
  return !lastLoadedParams.value || JSON.stringify(current) !== JSON.stringify(lastLoadedParams.value);
}

function updateLastLoadedParams() {
  lastLoadedParams.value = { coin: selectedCoin.value, exchange: selectedExchange.value, period: selectedPeriod.value };
}

function handleControlChange() {
  if (shouldReload()) {
    updateLastLoadedParams();
    reloadAll();
  }
}

const handleCoinChange = handleControlChange;
const handleExchangeChange = handleControlChange;
const handlePeriodChange = handleControlChange;

/* ---------- Utility Functions ---------- */
function formatPercent(valueLike: unknown) {
  return `${(Number(valueLike || 0) * 100).toFixed(1)}%`;
}

// Watch for data changes and increment version to force chart re-renders
watch([priceSeries, returnsSeries, volSeries, volumeSeries, signalsBySymbol], () => {
  dataVersion.value++;
}, { deep: true });

// Also watch for loading state changes
watch([loading], ([newLoading]) => {
  if (!newLoading) {
    // Small delay to ensure all data is properly set
    setTimeout(() => {
      dataVersion.value++;
    }, 100);
  }
});

watch([priceSeries, signalsBySymbol], () => {
  if (priceSeries.value.length > 0 || Object.keys(signalsBySymbol.value).length > 0) {
    updateLastLoadedParams();
  }
}, { immediate: true });
</script>