<template>
  <div class="zscore-controls">
    <div class="control-group">
      <label>Base</label>
      <select 
        :value="baseCoin" 
        @change="updateBaseCoin"
      >
        <option value="BTC">BTC</option>
        <option value="ETH">ETH</option>
        <option value="SOL">SOL</option>
        <option value="AVAX">AVAX</option>
      </select>
    </div>

    <div class="control-group" v-if="showCompareCoin">
      <label>vs</label>
      <select 
        :value="compareCoin" 
        @change="updateCompareCoin"
      >
        <option value="">None</option>
        <option value="ETH">ETH</option>
        <option value="BTC">BTC</option>
        <option value="SOL">SOL</option>
      </select>
    </div>

    <div class="control-group">
      <label>Timeframe</label>
      <select 
        :value="timeframe" 
        @change="updateTimeframe"
      >
        <option value="1h">1H</option>
        <option value="4h">4H</option>
        <option value="1d">1D</option>
      </select>
    </div>

    <button @click="$emit('update')" class="update-btn">
      Update
    </button>
    </div>
</template>

<script setup lang="ts">
interface Props {
  baseCoin: string
  compareCoin?: string
  timeframe: string
  showCompareCoin?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  showCompareCoin: true
})

const emit = defineEmits<{
  'update:baseCoin': [value: string]
  'update:compareCoin': [value: string]
  'update:timeframe': [value: string]
  'update': []
}>()

function updateBaseCoin(event: Event) {
  const value = (event.target as HTMLSelectElement).value
  emit('update:baseCoin', value)
}

function updateCompareCoin(event: Event) {
  const value = (event.target as HTMLSelectElement).value
  emit('update:compareCoin', value)
}

function updateTimeframe(event: Event) {
  const value = (event.target as HTMLSelectElement).value
  emit('update:timeframe', value)
}
</script>