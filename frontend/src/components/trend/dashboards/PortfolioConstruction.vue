<template>
  <div class="portfolio-construction-dashboard">
    <div class="dashboard-header">
      <h2>Portfolio Construction & Risk Management</h2>
    </div>

    <div class="construction-layout">
      <div class="constraints-panel">
        <h3>Portfolio Constraints</h3>
        <div class="constraints-grid">
          <div class="constraint-item">
            <label>Max Position:</label>
            <input v-model.number="constraints.maxPosition" type="number" step="0.1" class="constraint-input" />
            <span class="constraint-unit">%</span>
          </div>
          <div class="constraint-item">
            <label>Max Long:</label>
            <input v-model.number="constraints.maxLong" type="number" step="1" class="constraint-input" />
            <span class="constraint-unit">%</span>
          </div>
          <div class="constraint-item">
            <label>Max Short:</label>
            <input v-model.number="constraints.maxShort" type="number" step="1" class="constraint-input" />
            <span class="constraint-unit">%</span>
          </div>
          <div class="constraint-item">
            <label>Avg Vol Target:</label>
            <input v-model.number="constraints.avgVolTarget" type="number" step="0.1" class="constraint-input" />
            <span class="constraint-unit">%</span>
          </div>
        </div>
      </div>

      <div class="positions-section">
        <h3>Current Positions</h3>
        <div class="positions-group">
          <h4>Long Positions</h4>
          <div class="positions-list">
            <div v-for="position in longPositions" :key="position.asset" class="position-item long-position">
              <div class="position-header">
                <span class="asset-name">{{ position.asset }}</span>
                <span class="position-weight">{{ position.weight.toFixed(1) }}%</span>
              </div>
              <div class="position-details">
                <span class="position-vol">({{ position.vol.toFixed(2) }} vol)</span>
                <span class="position-signal">Signal: {{ position.signal.toFixed(2) }}</span>
              </div>
              <div class="position-bar">
                <div class="position-fill" :style="{ width: (position.weight / constraints.maxPosition * 100) + '%', backgroundColor: '#00BF63' }"></div>
              </div>
            </div>
          </div>
        </div>
        <div class="positions-group">
          <h4>Short Positions</h4>
          <div class="positions-list">
            <div v-for="position in shortPositions" :key="position.asset" class="position-item short-position">
              <div class="position-header">
                <span class="asset-name">{{ position.asset }}</span>
                <span class="position-weight">{{ position.weight.toFixed(1) }}%</span>
              </div>
              <div class="position-details">
                <span class="position-vol">({{ position.vol.toFixed(2) }} vol)</span>
                <span class="position-signal">Signal: {{ position.signal.toFixed(2) }}</span>
              </div>
              <div class="position-bar">
                <div class="position-fill" :style="{ width: (Math.abs(position.weight) / constraints.maxPosition * 100) + '%', backgroundColor: '#FF4757' }"></div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="correlation-section">
        <h3>Asset Correlations</h3>
        <div class="correlation-matrix">
          <div class="matrix-header" :style="{'--matrix-size': correlationAssets.length}">
            <div class="matrix-label"></div>
            <div v-for="asset in correlationAssets" :key="asset" class="matrix-header-cell">{{ asset }}</div>
          </div>
          <div v-for="(row, i) in correlationMatrix" :key="i" class="matrix-row" :style="{'--matrix-size': correlationAssets.length}">
            <div class="matrix-row-label">{{ correlationAssets[i] }}</div>
            <div v-for="(corr, j) in row" :key="j" class="matrix-cell" :style="{ backgroundColor: getCorrelationColor(corr) }" :title="`${correlationAssets[i]} vs ${correlationAssets[j]}: ${corr.toFixed(2)}`">
              {{ corr.toFixed(1) }}
            </div>
          </div>
        </div>
      </div>

      <div class="covariance-section">
        <h3>Covariance Matrix</h3>
        <div class="covariance-matrix">
          <div class="matrix-header" :style="{'--matrix-size': correlationAssets.length}">
            <div class="matrix-label"></div>
            <div v-for="asset in correlationAssets" :key="asset" class="matrix-header-cell">{{ asset }}</div>
          </div>
          <div v-for="(row, i) in covarianceMatrix" :key="i" class="matrix-row" :style="{'--matrix-size': correlationAssets.length}">
            <div class="matrix-row-label">{{ correlationAssets[i] }}</div>
            <div v-for="(cov, j) in row" :key="j" class="matrix-cell" :style="{ backgroundColor: getCovarianceColor(cov) }" :title="`${correlationAssets[i]} vs ${correlationAssets[j]}: ${(cov/1000).toFixed(3)}`">
              {{ cov.toFixed(1) }}
            </div>
          </div>
        </div>
      </div>

      <!-- Separate Portfolio Metrics Panel -->
      <div class="portfolio-metrics-panel">
        <h3>Portfolio Metrics</h3>
        <div class="metrics-grid">
          <div class="metric-card">
            <div class="metric-label">Total Long</div>
            <div class="metric-value long">{{ totalLong.toFixed(1) }}%</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Total Short</div>
            <div class="metric-value short">{{ totalShort.toFixed(1) }}%</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Net Exposure</div>
            <div class="metric-value net">{{ netExposure.toFixed(1) }}%</div>
          </div>
          <div class="metric-card">
            <div class="metric-label">Gross Exposure</div>
            <div class="metric-value gross">{{ grossExposure.toFixed(1) }}%</div>
          </div>
        </div>
      </div>

      <!-- Separate Expected Performance Panel -->
      <div class="expected-performance-panel">
        <h3>Expected Performance</h3>
        <div class="performance-grid">
          <div class="performance-card">
            <div class="perf-label">Daily Expected Return</div>
            <div class="perf-value positive">{{ expectedReturn.toFixed(1) }}%</div>
          </div>
          <div class="performance-card">
            <div class="perf-label">Daily Expected Vol</div>
            <div class="perf-value neutral">{{ expectedVol.toFixed(1) }}%</div>
          </div>
          <div class="performance-card">
            <div class="perf-label">Expected Sharpe</div>
            <div class="perf-value" :class="sharpeClass">{{ expectedSharpe.toFixed(1) }}</div>
          </div>
          <div class="performance-card">
            <div class="perf-label">Max Drawdown Risk</div>
            <div class="perf-value negative">{{ maxDrawdownRisk.toFixed(1) }}%</div>
          </div>
        </div>
      </div>

      <div class="attribution-panel">
        <div class="signal-attribution">
          <h3>Attribution by Signal</h3>
          <div class="attribution-bars">
            <div v-for="attr in signalAttribution" :key="attr.signal" class="attribution-bar">
              <div class="attr-label">{{ attr.signal }}</div>
              <div class="attr-bar-container">
                <div class="attr-bar-fill" :style="{ width: (Math.abs(attr.contribution) / maxSignalContribution) * 100 + '%', backgroundColor: attr.contribution > 0 ? '#00BF63' : '#FF4757' }"></div>
              </div>
              <div class="attr-value">{{ attr.contribution > 0 ? '+' : '' }}{{ attr.contribution.toFixed(1) }}%</div>
            </div>
          </div>
        </div>
        <div class="position-attribution">
          <h3>Attribution by Position</h3>
          <div class="position-perf-summary">
            <div class="perf-summary-item">
              <span class="summary-label">Longs Average:</span>
              <span class="summary-value positive">{{ longsAvgPerf.toFixed(1) }}%</span>
            </div>
            <div class="perf-summary-item">
              <span class="summary-label">Shorts Average:</span>
              <span class="summary-value negative">{{ shortsAvgPerf.toFixed(1) }}%</span>
            </div>
            <div class="perf-summary-item">
              <span class="summary-label">Net Contribution:</span>
              <span class="summary-value" :class="netContribClass">{{ netContribution.toFixed(1) }}%</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, inject } from 'vue'

// Inject shared state
// const signalData = inject('signalData')! // Assuming this might be used later

// Portfolio constraints
const constraints = ref({
  maxPosition: 5.0,
  maxLong: 100.0,
  maxShort: -50.0,
  avgVolTarget: 1.5
})

// Position data
const longPositions = ref([
  { asset: 'BTC', weight: 4.2, vol: 0.85, signal: 1.23 },
  { asset: 'ETH', weight: 3.8, vol: 0.92, signal: 0.98 },
  { asset: 'SOL', weight: 2.1, vol: 1.15, signal: 1.45 },
  { asset: 'ADA', weight: 1.8, vol: 1.05, signal: 0.76 },
  { asset: 'DOT', weight: 1.6, vol: 1.12, signal: 0.89 }
])

const shortPositions = ref([
  { asset: 'DOGE', weight: -2.1, vol: 1.80, signal: -1.34 },
  { asset: 'SHIB', weight: -1.5, vol: 2.20, signal: -0.98 },
  { asset: 'LTC', weight: -1.2, vol: 1.45, signal: -0.67 }
])

// Correlation matrix
const correlationAssets = ['BTC', 'ETH', 'SOL', 'ADA']
const correlationMatrix = [
  [1.0, 0.8, 0.7, 0.6],
  [0.8, 1.0, 0.9, 0.7],
  [0.7, 0.9, 1.0, 0.8],
  [0.6, 0.7, 0.8, 1.0]
]

// Covariance matrix (scaled by 1000 for display)
const covarianceMatrix = [
  [12.5, 8.2, 6.8, 5.1],
  [8.2, 15.6, 11.3, 7.9],
  [6.8, 11.3, 18.2, 9.4],
  [5.1, 7.9, 9.4, 13.7]
]

// Portfolio metrics
const totalLong = computed(() => longPositions.value.reduce((sum, pos) => sum + pos.weight, 0))
const totalShort = computed(() => Math.abs(shortPositions.value.reduce((sum, pos) => sum + pos.weight, 0)))
const netExposure = computed(() => totalLong.value - totalShort.value)
const grossExposure = computed(() => totalLong.value + totalShort.value)

// Expected performance
const expectedReturn = ref(0.8)
const expectedVol = ref(2.1)
const expectedSharpe = computed(() => expectedReturn.value / expectedVol.value * Math.sqrt(252))
const maxDrawdownRisk = ref(-8.5)
const sharpeClass = computed(() => expectedSharpe.value > 1.0 ? 'positive' : 'neutral')

// Attribution
const signalAttribution = ref([
  { signal: 'EWMAC', contribution: 0.3 },
  { signal: 'Momentum', contribution: 0.2 },
  { signal: 'Breakout', contribution: 0.1 },
  { signal: 'Trend', contribution: -0.1 }
])
const maxSignalContribution = computed(() => {
  if (!signalAttribution.value.length) return 1;
  return Math.max(...signalAttribution.value.map(attr => Math.abs(attr.contribution)));
});
const longsAvgPerf = ref(0.4)
const shortsAvgPerf = ref(-0.1)
const netContribution = computed(() => longsAvgPerf.value + shortsAvgPerf.value)
const netContribClass = computed(() => netContribution.value > 0 ? 'positive' : 'negative')

// Utility functions
const getCorrelationColor = (corr: number) => {
  const intensity = Math.abs(corr);
  const hue = corr > 0 ? 145 : 0; // Green for positive, Red for negative
  const saturation = 70;
  const lightness = 50;
  return `hsla(${hue}, ${saturation}%, ${lightness}%, ${intensity * 0.8 + 0.2})`;
}

const getCovarianceColor = (cov: number) => {
  const maxCov = 20; // Approximate max covariance for scaling
  const intensity = Math.min(Math.abs(cov) / maxCov, 1);
  const hue = 220; // Blue color for covariance
  const saturation = 60;
  const lightness = 45;
  return `hsla(${hue}, ${saturation}%, ${lightness}%, ${intensity * 0.7 + 0.3})`;
}
</script>