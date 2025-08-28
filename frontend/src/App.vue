<template>
  <div class="app">
    <header class="top-nav">
      <div v-if="route.path === '/funding'" class="brand">
        Funding Dashboard
      </div>
      
      <nav class="tabs" :class="{ 'tabs-alone': route.path !== '/funding' }">
        <RouterLink to="/funding" class="tab">Funding</RouterLink>
        <RouterLink to="/trends" class="tab">Trends</RouterLink>
      </nav>
    </header>

    <main>
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router';

const route = useRoute();
</script>

<style scoped>
.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

main {
  flex: 1;
}
</style>