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
            {{ displayMode === 'annualized' ? 'APR (3x daily payments)' : '8-hour funding rate' }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import FundingTable from './FundingTable.vue'
import { fetchFundingRates, handleApiError } from '../utils/api'
import { formatTimestamp } from '../utils/formatters'
import { sumOpenInterest, calculateArbSpread, findBestArbOpportunity } from '../utils/calculations'
import { REFRESH_INTERVAL } from '../utils/constants'
import type { TokenRow, DisplayMode, SpreadUnit, SortDirection } from '../utils/types'

// State
const tokens = ref<TokenRow[]>([])
const lastUpdated = ref<string>('')
const exchanges = ref<string[]>([])
const isLoading = ref(false)
const error = ref<string | null>(null)

// Display options
const displayMode = ref<DisplayMode>('rate')
const spreadUnit = ref<SpreadUnit>('percentage')

// Sorting state
const sortColumn = ref<string>('')
const sortDirection = ref<SortDirection>('desc')

// Refresh interval handle
let refreshInterval: number | undefined

// Computed property for sorted tokens
const sortedTokens = computed(() => {
  const tokensCopy = [...tokens.value]
  
  if (!sortColumn.value) return tokensCopy
  
  return tokensCopy.sort((a, b) => {
    let aValue: any
    let bValue: any
    
    if (sortColumn.value === 'token') {
      aValue = a.token
      bValue = b.token
    } else if (sortColumn.value === 'openInterest') {
      aValue = sumOpenInterest(a.exchanges)
      bValue = sumOpenInterest(b.exchanges)
    } else if (sortColumn.value === 'arbSpread') {
      aValue = calculateArbSpread(a.exchanges) ?? -1
      bValue = calculateArbSpread(b.exchanges) ?? -1
    } else if (sortColumn.value === 'arbCombo') {
      aValue = findBestArbOpportunity(a.exchanges)?.spread ?? -1
      bValue = findBestArbOpportunity(b.exchanges)?.spread ?? -1
    } else if (sortColumn.value === 'arbOI') {
      aValue = findBestArbOpportunity(a.exchanges)?.combinedOI ?? -1
      bValue = findBestArbOpportunity(b.exchanges)?.combinedOI ?? -1
    } else if (sortColumn.value.startsWith('exchange_')) {
      const exchange = sortColumn.value.replace('exchange_', '')
      aValue = a.exchanges[exchange]?.funding_rate ?? -999
      bValue = b.exchanges[exchange]?.funding_rate ?? -999
    }
    
    if (aValue < bValue) return sortDirection.value === 'asc' ? -1 : 1
    if (aValue > bValue) return sortDirection.value === 'asc' ? 1 : -1
    return 0
  })
})

async function fetchData() {
  try {
    if (!tokens.value.length) {
      isLoading.value = true
    }
    error.value = null
    
    const response = await fetchFundingRates()
    
    tokens.value = response.tokens
    lastUpdated.value = response.last_updated
    
    // Extract unique exchanges
    const allExchanges = new Set<string>()
    tokens.value.forEach(t => {
      Object.keys(t.exchanges).forEach(ex => allExchanges.add(ex))
    })
    exchanges.value = Array.from(allExchanges).sort()
    
  } catch (err) {
    error.value = handleApiError(err)
  } finally {
    isLoading.value = false
  }
}

function handleSort(column: string) {
  if (sortColumn.value === column) {
    sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortColumn.value = column
    sortDirection.value = 'desc'
  }
}

onMounted(() => {
  fetchData()
  refreshInterval = window.setInterval(fetchData, REFRESH_INTERVAL)
})

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
  }
})
</script>