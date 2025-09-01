<template>
  <div>
    <!-- Page Header -->
    <header class="page-header">
      <h1 class="page-title">Funding Rates Arbitrage</h1>
      <p class="page-subtitle">Cross-Dex</p>
    </header>

    <!-- Dashboard Controls -->
    <div class="dashboard-controls">
      <div class="control-group">
        <label>Display Mode:</label>
        <div class="toggle-buttons">
          <button 
            @click="displayMode = 'rate'" 
            :class="['toggle-btn', { active: displayMode === 'rate' }]"
          >
            8h Rate
          </button>
          <button 
            @click="displayMode = 'annualized'" 
            :class="['toggle-btn', { active: displayMode === 'annualized' }]"
          >
            Annualized
          </button>
        </div>
      </div>
      
      <div class="control-group">
        <label>Spread Unit:</label>
        <div class="toggle-buttons">
          <button 
            @click="spreadUnit = 'percentage'" 
            :class="['toggle-btn', { active: spreadUnit === 'percentage' }]"
          >
            Percentage
          </button>
          <button 
            @click="spreadUnit = 'bps'" 
            :class="['toggle-btn', { active: spreadUnit === 'bps' }]"
          >
            Basis Points
          </button>
        </div>
      </div>
    </div>

    <!-- Dashboard -->
    <div class="dashboard">
      <!-- Loading State -->
      <div v-if="isLoading && !tokens.length" class="loading-container">
        <div class="loading-spinner"></div>
        <p>Loading funding rates...</p>
      </div>

      <!-- Error State -->
      <div v-else-if="error" class="error-container">
        <p class="error-message">⚠️ {{ error }}</p>
        <button @click="fetchData" class="retry-button">Retry</button>
      </div>

      <!-- Data Table -->
      <div v-else class="dashboard-card">
        <FundingTable 
          :tokens="sortedTokens"
          :exchanges="exchanges"
          :display-mode="displayMode"
          :spread-unit="spreadUnit"
          :sort-column="sortColumn"
          :sort-direction="sortDirection"
          @sort="handleSort"
        />
        
        <div class="last-updated">
          <span v-if="lastUpdated">
            Last updated: {{ formatTimestamp(lastUpdated) }}
          </span>
          <span v-if="isLoading" class="updating-indicator">
            Updating...
          </span>
          <span class="rate-info">
            {{ displayMode === 'annualized' ? '(3x daily payments)' : '8-hour funding rate' }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import FundingTable from './FundingTable.vue'
import { formatTimestamp } from '../../utils/formatters'
import {
  tokens,
  lastUpdated,
  exchanges,
  isLoading,
  error,
  displayMode,
  spreadUnit,
  sortColumn,
  sortDirection,
  sortedTokens,
  fetchData,
  handleSort
} from './FundingLogic'
</script>
