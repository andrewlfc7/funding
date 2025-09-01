<template>
  <section class="page zscore-wrapper">
    <ZScoreDashboard />
  </section>
</template>

<script setup lang="ts">
import ZScoreDashboard from './zscore/ZScoreDashboard.vue'
</script>

<style scoped>
.zscore-wrapper {
  display: flex;
  width: 100%;
  height: 100%;
  overflow: hidden;
}
</style>