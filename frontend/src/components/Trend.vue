<!-- src/components/Trend.vue -->
<template>
  <section class="page trend-wrapper">
    <TrendDashboard />
  </section>
</template>

<script setup lang="ts">
import TrendDashboard from './trend/TrendDashboard.vue'
</script>


<style scoped>
.page.trend-wrapper {
  width: 100%;
  height: 100vh;
  overflow: hidden;
  padding: 0;
  margin: 0;
}
</style>