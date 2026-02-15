<template>
  <div class="combined-chart-container">
    <div class="chart-header">
      <h4>Combined Signals</h4>
      <div class="signal-toggles">
        <button
          v-for="signal in availableSignals"
          :key="signal.id"
          @click="toggleSignal(signal.id)"
          :class="['signal-toggle', { active: activeSignals[signal.id] }]"
          :style="{ 
            borderColor: activeSignals[signal.id] ? signal.color : '#34495E',
            color: activeSignals[signal.id] ? signal.color : '#BDC3C7'
          }"
        >
          {{ signal.name }}
        </button>
      </div>
    </div>
    
    <div v-if="loading" class="loading-state">
      <div class="spinner"></div>
      <span>Loading signals...</span>
    </div>
    
    <div v-else-if="error" class="error-state">
      <span class="error-icon">⚠</span>
      <span>{{ error }}</span>
    </div>
    
    <div v-else class="chart-wrapper">
      <!-- Use TimeSeriesChart component instead of canvas -->
      <TimeSeriesChart
        :series="chartSeries"
        :height="chartHeight"
        y-label="Signal Strength"
        :y-format="(value) => value.toFixed(2)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import TimeSeriesChart from '@/components/zscore/components/charts/TimeSeriesChart.vue'

interface Signal {
  id: string
  name: string
  color: string
}

interface Props {
  labels: Date[]
  dataById: Record<string, number[]>
  signals?: Signal[]
  activeSignals?: Record<string, boolean>
  loading?: boolean
  error?: string | null
  height?: number
}

const props = withDefaults(defineProps<Props>(), {
  signals: () => [
    { id: 'momentum', name: 'Momentum', color: '#00BF63' },
    { id: 'ewmac', name: 'EWMAC', color: '#00D4FF' },
    { id: 'breakout', name: 'Breakout', color: '#FF6B6B' },
    { id: 'trend', name: 'Trend', color: '#7C5CFF' },
    { id: 'composite', name: 'Composite', color: '#FFA502' }
  ],
  activeSignals: () => ({
    momentum: true,
    ewmac: true,
    breakout: true,
    trend: true,
    composite: true
  }),
  loading: false,
  error: null,
  height: 300
})

const emit = defineEmits<{
  'update:activeSignals': [signals: Record<string, boolean>]
}>()

const availableSignals = computed(() => props.signals!)
const localActiveSignals = ref({ ...props.activeSignals! })
const chartHeight = computed(() => props.height)

// Convert data to TimeSeriesChart format
const chartSeries = computed(() => {
  return availableSignals.value
    .filter(signal => localActiveSignals.value[signal.id])
    .map(signal => {
      const signalData = props.dataById[signal.id] || []
      
      return {
        symbol: signal.name,
        data: signalData.map((value, index) => ({
          timestamp: props.labels[index]?.getTime() || 0,
          value: value
        }))
      }
    })
})

function toggleSignal(signalId: string) {
  localActiveSignals.value[signalId] = !localActiveSignals.value[signalId]
  emit('update:activeSignals', { ...localActiveSignals.value })
}

// Watch for external changes to activeSignals
watch(() => props.activeSignals, (newSignals) => {
  if (newSignals) {
    localActiveSignals.value = { ...newSignals }
  }
}, { deep: true })
</script>
