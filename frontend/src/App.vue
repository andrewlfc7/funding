<template>
  <div class="app">
    <header class="top-nav">
      <div class="nav-content">
        <div class="brand">
          {{ currentPageTitle }}
        </div>
        
        <nav class="tabs">
          <RouterLink to="/funding" class="tab">Funding</RouterLink>
          <RouterLink to="/trend" class="tab">Trend</RouterLink>
          <RouterLink to="/zscore" class="tab">Z-Score</RouterLink>
        </nav>
      </div>
    </header>

    <main>
      <RouterView />
    </main>
  </div>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router';
import { computed } from 'vue';

const route = useRoute();

const currentPageTitle = computed(() => {
  switch(route.path) {
    case '/funding': return 'Funding Dashboard';
    case '/trend': return 'Trend Analysis';
    case '/zscore': return 'Z-Score Analysis';
    default: return 'Dashboard';
  }
});
</script>

<style scoped>
.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.top-nav {
  position: sticky;
  top: 0;
  z-index: 100;
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border-primary);
  height: 60px;
}

.nav-content {
  max-width: 100%;
  height: 100%;
  padding: 0 var(--space-xl);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.brand {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-bright);
}

main {
  flex: 1;
  display: flex;
  overflow: hidden;
}
</style>