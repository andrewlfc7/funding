<!-- frontend/src/components/trend/Controls.vue -->
<template>
  <div class="controls">
    <div class="control-group">
      <label>Coin:</label>
      <select :value="coin" @change="$emit('update:coin', ($event.target as HTMLSelectElement).value); $emit('update')">
        <option v-for="c in availableCoins" :key="c" :value="c">
          {{ c }}
        </option>
      </select>
    </div>
    
    <div class="control-group">
      <label>Exchange:</label>
      <select :value="exchange" @change="$emit('update:exchange', ($event.target as HTMLSelectElement).value); $emit('update')">
        <option v-for="ex in availableExchanges" :key="ex" :value="ex">
          {{ ex }}
        </option>
      </select>
    </div>
    
    <div class="control-group">
      <label>Period:</label>
      <select :value="period" @change="$emit('update:period', ($event.target as HTMLSelectElement).value); $emit('update')">
        <option value="30d">30 Days</option>
        <option value="90d">90 Days</option>
        <option value="180d">180 Days</option>
        <option value="1y">1 Year</option>
      </select>
    </div>
    
    <div class="control-group signal-view-toggle">
      <label>View:</label>
      <div class="toggle-buttons">
        <button 
          :class="['toggle-btn', { active: viewMode === 'combined' }]"
          @click="$emit('update:viewMode', 'combined'); $emit('update')"
        >
          Combined
        </button>
        <button 
          :class="['toggle-btn', { active: viewMode === 'individual' }]"
          @click="$emit('update:viewMode', 'individual'); $emit('update')"
        >
          Individual
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
interface Props {
  coin: string
  exchange: string
  period: string
  viewMode: 'combined' | 'individual'
  availableCoins: string[]
  availableExchanges: string[]
}

defineProps<Props>()

defineEmits<{
  'update:coin': [value: string]
  'update:exchange': [value: string]
  'update:period': [value: string]
  'update:viewMode': [value: 'combined' | 'individual']
  'update': []
}>()
</script>

