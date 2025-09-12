<template>
  <div class="zscore-dashboard">
    <!-- Sub-navigation sidebar -->
    <nav class="dashboard-nav">
      <button 
        v-for="(dashboard, index) in dashboards" 
        :key="index"
        @click="currentDashboard = index"
        :class="['nav-btn', { active: currentDashboard === index }]"
        :title="dashboard.name"
      >
        <span class="nav-number">{{ index + 1 }}</span>
        <span class="nav-tooltip">{{ dashboard.name }}</span>
      </button>
    </nav>

    <!-- Dashboard content area -->
    <div class="dashboard-content">
      <!-- Dashboard 1: Z-Score Overview -->
      <ZScoreOverview v-if="currentDashboard === 0" />
      
      <!-- Dashboard 2: Volatility Analysis -->
      <VolatilityAnalysis v-if="currentDashboard === 1" />
      
      <!-- Dashboard 3: Cross-Asset Matrix -->
      <CrossAssetMatrix v-if="currentDashboard === 2" />
      
      <!-- Dashboard 4: Volatility & Liquidity -->
      <VolatilityLiquidity v-if="currentDashboard === 3" />
      
      <!-- Dashboard 5: Inter-Asset Z-Score -->
      <InterAssetZScore v-if="currentDashboard === 4" />
      
      <!-- Dashboard 6: Leaders & Laggards -->
      <LeadersLaggards v-if="currentDashboard === 5" />
      
      <!-- Dashboard 7: Market Microstructure -->
      <MarketMicrostructure v-if="currentDashboard === 6" />
      
      <!-- Dashboard 8: Relative Strength -->
      <RelativeStrength v-if="currentDashboard === 7" />
      
      <!-- Dashboard 9: Market Regime -->
      <MarketRegime v-if="currentDashboard === 8" />

      <!-- Dashboard 10:  TradesAnalysis -->
      <TradesAnalysis v-if="currentDashboard === 9 " />

      <!-- Dashboard 10:  TradesAnalysis -->
      <VolatilityDynamics v-if="currentDashboard === 10 " />

      
      <!-- Dashboard 10:  TradesAnalysis -->
      <MarketSeasonality v-if="currentDashboard === 11 " />


    </div>
  </div>
</template>




<script setup lang="ts">
import { ref, provide } from 'vue'
import ZScoreOverview from './dashboards/ZScoreOverview.vue'
import VolatilityAnalysis from './dashboards/VolatilityAnalysis.vue'
import CrossAssetMatrix from './dashboards/CrossAssetMatrix.vue'
import VolatilityLiquidity from './dashboards/VolatilityLiquidity.vue'
import InterAssetZScore from './dashboards/InterAssetZScore.vue'
import LeadersLaggards from './dashboards/LeadersLaggards.vue'
import MarketMicrostructure from './dashboards/MarketMicrostructure.vue'
import RelativeStrength from './dashboards/RelativeStrength.vue'
import MarketRegime from './dashboards/MarketRegime.vue'

import TradesAnalysis from './dashboards/TradesAnalysis.vue'
import VolatilityDynamics from './dashboards/VolatilityDynamics.vue'
import MarketSeasonality from './dashboards/MarketSeasonality.vue'


const dashboards = [
  { name: 'Z-Score Overview' },
  { name: 'Volatility Analysis' },    
  { name: 'Cross-Asset Matrix' },
  { name: 'Volatility & Liquidity' },
  { name: 'Inter-Asset Z-Score' },
  { name: 'Leaders & Laggards' },
  { name: 'Market Microstructure' },
  { name: 'Relative Strength' },
  { name: 'Market Regime' },
  { name: 'Trades Analysis' },     
  { name: 'Volatility Dynamics' }, 
  { name: 'Market Seasonality' },       

];


// Current dashboard state
const currentDashboard = ref(0)

// Provide common data to all dashboards
const selectedExchange = ref('binance')
const selectedPeriod = ref('24h')

provide('exchange', selectedExchange)
provide('period', selectedPeriod)
</script>