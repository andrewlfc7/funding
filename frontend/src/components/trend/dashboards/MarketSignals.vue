<template>
  <div class="market-signals-dashboard">
    <div class="dashboard-controls">
      <div class="control-group">
        <label>Asset:</label>
        <select v-model="selectedCoin" @change="reloadAll" class="control-select">
          <option v-for="symbol in symbols" :key="symbol" :value="symbol">
            {{ symbol }}
          </option>
        </select>
      </div>
      
      <div class="control-group">
        <label>Exchange:</label>
        <select v-model="selectedExchange" @change="reloadAll" class="control-select">
          <option value="binance">Binance</option>
          <option value="coinbase">Coinbase</option>
          <option value="kraken">Kraken</option>
        </select>
      </div>
      
      <div class="control-group">
        <label>Period:</label>
        <select v-model="selectedPeriod" @change="reloadAll" class="control-select">
          <option value="30d">30 Days</option>
          <option value="60d">60 Days</option>
          <option value="90d">90 Days</option>
        </select>
      </div>
    </div>

    <!-- 2x3 Grid Layout -->
    <div class="signals-grid">
      <!-- Row 1: Price & Volume | Returns & Volatility -->
      <div class="grid-item price-vol-panel">
        <h3>Price & Volume</h3>
        <div class="panel-content">
          <div class="price-chart-container">
            <PriceChart
              :labels="chartLabels"
              :prices="priceData"
              :volumes="volumeData"
              :loading="loadingLive || loadingXSec"
              :error="errorLive || errorXSec"
            />
          </div>
          <div class="volume-stats">
            <div class="stat-item">
              <span class="label">20d EWMA Vol:</span>
              <span class="value">{{ formatVolume(avgVolume) }}</span>
            </div>
            <div class="stat-item">
              <span class="label">Current Price:</span>
              <span class="value">${{ formatPrice(currentPrice) }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="grid-item returns-vol-panel">
        <h3>Returns & Volatility</h3>
        <div class="panel-content">
          <ReturnsChart
            :labels="chartLabels"
            :returns="returnsData"
            :volatility="volatilityData"
            :loading="loadingLive"
            :error="errorLive"
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

      <!-- Row 2: EWMAC Signal | Breakout Signal -->
      <div class="grid-item ewmac-panel">
        <h3>EWMAC Signal</h3>
        <div class="panel-content">
          <SignalChart
            title="EWMAC (4/16 span)"
            color="#00D4FF"
            :labels="chartLabels"
            :values="ewmacValues"
            :loading="loadingXSec"
            :error="errorXSec"
          />
          <div class="signal-distribution">
            <div class="dist-item">
              <span class="label">Raw Score:</span>
              <span class="value">{{ formatSignal(currentEWMAC) }}</span>
            </div>
            <div class="dist-item">
              <span class="label">Standardized:</span>
              <span class="value">{{ formatSignal(currentEWMACStd) }}</span>
            </div>
            <div class="dist-item">
              <span class="label">Rank:</span>
              <span class="value">{{ ewmacRank }}/100</span>
            </div>
          </div>
        </div>
      </div>

      <div class="grid-item breakout-panel">
        <h3>Breakout Signal</h3>
        <div class="panel-content">
          <SignalChart
            title="Breakout Score (20d)"
            color="#FF6B6B"
            :labels="chartLabels"
            :values="breakoutValues"
            :loading="loadingXSec"
            :error="errorXSec"
          />
          <div class="breakout-metrics">
            <div class="metric-item">
              <span class="label">Days from High:</span>
              <span class="value">{{ daysFromHigh }}d</span>
            </div>
            <div class="metric-item">
              <span class="label">Cross-sec Rank:</span>
              <span class="value">{{ breakoutRank }}/100</span>
            </div>
            <div class="metric-item">
              <span class="label">Strength:</span>
              <div class="strength-indicator" :class="getStrengthClass(currentBreakout)">
                {{ getStrengthLabel(currentBreakout) }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Row 3: Momentum Signal | Combined Signals -->
      <div class="grid-item momentum-panel">
        <h3>Momentum Signal</h3>
        <div class="panel-content">
          <SignalChart
            title="Momentum (20d, 5d HL)"
            color="#00BF63"
            :labels="chartLabels"
            :values="momentumValues"
            :loading="loadingXSec"
            :error="errorXSec"
          />
          <div class="momentum-percentiles">
            <div class="perc-item">
              <span class="label">Weighted Returns:</span>
              <span class="value">{{ formatPercent(weightedReturns) }}</span>
            </div>
            <div class="perc-item">
              <span class="label">Mom Score:</span>
              <span class="value">{{ formatSignal(currentMomentum) }}</span>
            </div>
            <div class="perc-item">
              <span class="label">Percentile:</span>
              <span class="value">{{ momentumPercentile }}%</span>
            </div>
          </div>
        </div>
      </div>

      <div class="grid-item combined-panel">
        <h3>Combined Signals</h3>
        <div class="panel-content">
          <CombinedChart
            :labels="chartLabels"
            :dataById="signalDataById"
            :activeSignals="activeSignals"
            @update:activeSignals="activeSignals = $event"
            :loading="loadingXSec"
            :error="errorXSec"
          />
          <div class="correlation-mini-matrix">
            <div class="matrix-title">Signal Correlations</div>
            <div class="mini-matrix">
              <div v-for="(row, i) in signalCorrelations" :key="i" class="matrix-row">
                <div 
                  v-for="(corr, j) in row" 
                  :key="j"
                  class="matrix-cell"
                  :style="{ backgroundColor: getCorrelationColor(corr) }"
                  :title="`${signalNames[i]} vs ${signalNames[j]}: ${corr.toFixed(2)}`"
                >
                  {{ corr.toFixed(1) }}
                </div>
              </div>
            </div>
          </div>
          <div class="composite-score">
            <div class="score-label">Composite Score:</div>
            <div class="score-value" :class="getScoreClass(currentComposite)">
              {{ formatSignal(currentComposite) }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
// FIX: Use `import type` for type-only imports
import { ref, computed, inject, onMounted, type Ref } from 'vue'
import PriceChart from '../components/PriceChart.vue'
import ReturnsChart from '../components/ReturnsChart.vue'
import SignalChart from '../components/SignalChart.vue'
import CombinedChart from '../components/CombinedChart.vue'

// --- TYPE DEFINITIONS ---

interface DataPoint {
  timestamp: number;
  price: number;
  volume: number;
  returns: number;
  volatility: number;
  momentum: number;
  ewmac: number;
  breakout: number;
  composite: number;
}

interface Controls {
  selectedCoin: Ref<string>;
  selectedExchange: Ref<string>;
  selectedPeriod: Ref<string>;
  reloadAll: () => void;
}

interface MarketData {
  klines: Ref<any[]>;
  returnsSeries: Ref<number[]>;
  volSeries: Ref<number[]>;
  loadingLive: Ref<boolean>;
  errorLive: Ref<string | null>;
}

interface SignalData {
  xsecSignals: Ref<Record<string, any>>;
  loadingXSec: Ref<boolean>;
  errorXSec: Ref<string | null>;
  series: Ref<Record<string, DataPoint[]>>;
  symbols: Ref<string[]>;
}

// FIX: This type can be simplified as it's passed to a component
// that accepts a more generic record.
type ActiveSignals = Record<string, boolean>;

// --- INJECTIONS ---

// Inject shared state with proper typing
const controls = inject<Controls>('controls')!
const marketData = inject<MarketData>('marketData')!
const signalData = inject<SignalData>('signalData')!

const { selectedCoin, selectedExchange, selectedPeriod, reloadAll } = controls
const { klines, returnsSeries, volSeries, loadingLive, errorLive } = marketData
const { xsecSignals, loadingXSec, errorXSec, series, symbols } = signalData

// --- LOCAL STATE ---

// Local state for signal toggles with explicit type
const activeSignals = ref<ActiveSignals>({
  momentum: true,
  ewmac: true,
  breakout: true,
  composite: true
})

// --- COMPUTED PROPERTIES ---

// Chart data adapters
const chartLabels = computed(() => {
  const symbolData = series.value[selectedCoin.value]
  if (!symbolData) return []
  return symbolData.map((point: DataPoint) => new Date(point.timestamp * 1000))
})

const signalDataById = computed((): Record<string, number[]> => {
  const symbolData = series.value[selectedCoin.value]
  if (!symbolData || symbolData.length === 0) {
    return { momentum: [], ewmac: [], breakout: [], composite: [] }
  }
  
  return {
    momentum: symbolData.map((p: DataPoint) => p.momentum),
    ewmac: symbolData.map((p: DataPoint) => p.ewmac),
    breakout: symbolData.map((p: DataPoint) => p.breakout),
    composite: symbolData.map((p: DataPoint) => p.composite)
  }
})

const priceData = computed(() => {
  const symbolData = series.value[selectedCoin.value]
  if (!symbolData) return []
  return symbolData.map((p: DataPoint) => p.price)
})

const volumeData = computed(() => {
  const symbolData = series.value[selectedCoin.value]
  if (!symbolData) return []
  return symbolData.map((p: DataPoint) => p.volume)
})

const returnsData = computed(() => {
  const symbolData = series.value[selectedCoin.value]
  if (!symbolData) return []
  return symbolData.map((p: DataPoint) => p.returns)
})

const volatilityData = computed(() => {
  const symbolData = series.value[selectedCoin.value]
  if (!symbolData) return []
  return symbolData.map((p: DataPoint) => p.volatility)
})

// Individual signal values
const ewmacValues = computed(() => signalDataById.value.ewmac || [])
const breakoutValues = computed(() => signalDataById.value.breakout || [])
const momentumValues = computed(() => signalDataById.value.momentum || [])

// Current values (latest in time series)
const currentPrice = computed(() => {
  const prices = priceData.value
  return prices.length > 0 ? prices[prices.length - 1] : 0
})

const currentVol = computed(() => {
  const vols = volatilityData.value
  return vols.length > 0 ? vols[vols.length - 1] : 0
})

const currentEWMAC = computed(() => {
  const values = ewmacValues.value
  return values.length > 0 ? values[values.length - 1] : 0
})

const currentBreakout = computed(() => {
  const values = breakoutValues.value
  return values.length > 0 ? values[values.length - 1] : 0
})

const currentMomentum = computed(() => {
  const values = momentumValues.value
  return values.length > 0 ? values[values.length - 1] : 0
})

const currentComposite = computed(() => {
  const values = signalDataById.value.composite || []
  return values.length > 0 ? values[values.length - 1] : 0
})

// Derived metrics
const avgVolume = computed(() => {
  const volumes = volumeData.value
  if (volumes.length === 0) return 0
  return volumes.reduce((sum: number, vol: number) => sum + vol, 0) / volumes.length
})

const vol25th = computed(() => {
  const vols = [...volatilityData.value].sort((a: number, b: number) => a - b)
  if (vols.length === 0) return 0
  const index = Math.floor(vols.length * 0.25)
  return vols[index]
})

const vol75th = computed(() => {
  const vols = [...volatilityData.value].sort((a: number, b: number) => a - b)
  if (vols.length === 0) return 0
  const index = Math.floor(vols.length * 0.75)
  return vols[index]
})

// Mock derived values (would come from API in real implementation)
const currentEWMACStd = computed(() => currentEWMAC.value * 1.2)
const ewmacRank = computed(() => Math.floor(Math.random() * 100))
const daysFromHigh = computed(() => Math.floor(Math.random() * 30))
const breakoutRank = computed(() => Math.floor(Math.random() * 100))
const weightedReturns = computed(() => currentMomentum.value * 0.01)
const momentumPercentile = computed(() => Math.floor(Math.random() * 100))

// Signal correlations matrix
const signalNames = ['Mom', 'EWMAC', 'Break', 'Comp']
const signalCorrelations = computed(() => [
  [1.0, 0.3, 0.5, 0.8],
  [0.3, 1.0, 0.2, 0.7],
  [0.5, 0.2, 1.0, 0.6],
  [0.8, 0.7, 0.6, 1.0]
])

// --- UTILITY FUNCTIONS ---

function formatPrice(price: number): string {
  return price.toLocaleString('en-US', { 
    minimumFractionDigits: 2, 
    maximumFractionDigits: 2 
  })
}

function formatVolume(volume: number): string {
  if (volume > 1e9) return (volume / 1e9).toFixed(1) + 'B'
  if (volume > 1e6) return (volume / 1e6).toFixed(1) + 'M'
  if (volume > 1e3) return (volume / 1e3).toFixed(1) + 'K'
  return volume.toFixed(0)
}

function formatPercent(value: number): string {
  return (value * 100).toFixed(1) + '%'
}

function formatSignal(value: number): string {
  return value.toFixed(2)
}

function getStrengthClass(value: number): string {
  const abs = Math.abs(value)
  if (abs > 2) return 'strong'
  if (abs > 1) return 'medium'
  return 'weak'
}

function getStrengthLabel(value: number): string {
  const abs = Math.abs(value)
  if (abs > 2) return 'Strong'
  if (abs > 1) return 'Medium'
  return 'Weak'
}

function getScoreClass(value: number): string {
  if (value > 1) return 'bullish'
  if (value < -1) return 'bearish'
  return 'neutral'
}

function getCorrelationColor(corr: number): string {
  const intensity = Math.abs(corr)
  if (corr > 0) return `rgba(0, 191, 99, ${intensity})`
  return `rgba(255, 71, 87, ${intensity})`
}
</script>
