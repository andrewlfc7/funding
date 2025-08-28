<template>
  <div class="page-header">
    <h1 class="page-title">Market Trends</h1>
    <p class="page-subtitle">Historical funding rate trends and analytics</p>
  </div>
  
  <div class="dashboard">
    <div class="dashboard-card">
      <div class="trends-placeholder">
        <div class="placeholder-icon">📊</div>
        <h2>Trends Analysis Coming Soon</h2>
        <p>This section will display:</p>
        <ul>
          <li>Historical funding rate charts</li>
          <li>Volume trends across exchanges</li>
          <li>Arbitrage opportunity patterns</li>
          <li>Market sentiment indicators</li>
        </ul>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
// Add your trends logic here
</script>

<style scoped>
.trends-placeholder {
  padding: 60px 40px;
  text-align: center;
  color: #d0d0d0;
}

.placeholder-icon {
  font-size: 64px;
  margin-bottom: 24px;
  opacity: 0.5;
}

.trends-placeholder h2 {
  font-size: 24px;
  font-weight: 600;
  color: #f0f0f0;
  margin-bottom: 16px;
}

.trends-placeholder p {
  color: #999;
  margin-bottom: 24px;
  font-size: 16px;
}

.trends-placeholder ul {
  list-style: none;
  padding: 0;
  max-width: 400px;
  margin: 0 auto;
  text-align: left;
}

.trends-placeholder li {
  padding: 12px 0;
  border-bottom: 1px solid #1a1a1a;
  color: #888;
  font-size: 14px;
}

.trends-placeholder li:last-child {
  border-bottom: none;
}
</style>