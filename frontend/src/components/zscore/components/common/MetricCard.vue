<template>
  <div :class="['metric-card', { 'span-2': span === 2 }]">
    <div class="card-header" v-if="title || $slots.header">
      <h3 v-if="title">{{ title }}</h3>
      <slot name="header" />
    </div>
    
    <div class="card-content">
      <div v-if="loading" class="loading-state">
        <LoadingSpinner />
      </div>
      
      <div v-else-if="error" class="error-state">
        <p>{{ error }}</p>
        <button @click="$emit('retry')" class="retry-btn">Retry</button>
      </div>
      
      <div v-else-if="empty" class="empty-state">
        <p>No data available</p>
      </div>
      
      <slot v-else />
    </div>
  </div>
</template>

<script setup lang="ts">
import LoadingSpinner from './LoadingSpinner.vue'

interface Props {
  title?: string
  span?: 1 | 2
  loading?: boolean
  error?: string | null
  empty?: boolean
}

defineProps<Props>()
defineEmits<{
  retry: []
}>()
</script>