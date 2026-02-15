<template>
  <div class="funding-dashboard-wrapper">
    <!-- Page Header -->
    <header class="funding-page-header">
      <h1 class="funding-page-title">Funding Rates Arbitrage</h1>
      <p class="funding-page-subtitle">Cross-Dex Analysis</p>
    </header>

    <!-- Dashboard Controls -->
    <div class="funding-dashboard-controls">
      <div class="funding-control-group">
        <label>View Mode:</label>
        <div class="funding-view-toggle">
          <button 
            @click="viewMode = 'table'" 
            :class="['funding-view-btn', { active: viewMode === 'table' }]"
          >
            Table
          </button>
          <button 
            @click="viewMode = 'matrix'" 
            :class="['funding-view-btn', { active: viewMode === 'matrix' }]"
          >
            Matrix
          </button>
        </div>
      </div>

      <div class="funding-control-group">
        <label>Display Mode:</label>
        <div class="funding-toggle-buttons">
          <button 
            @click="displayMode = 'rate'" 
            :class="['funding-toggle-btn', { active: displayMode === 'rate' }]"
          >
            8h Rate
          </button>
          <button 
            @click="displayMode = 'annualized'" 
            :class="['funding-toggle-btn', { active: displayMode === 'annualized' }]"
          >
            Annualized
          </button>
        </div>
      </div>
      
      <div class="funding-control-group">
        <label>Spread Unit:</label>
        <div class="funding-toggle-buttons">
          <button 
            @click="spreadUnit = 'percentage'" 
            :class="['funding-toggle-btn', { active: spreadUnit === 'percentage' }]"
          >
            Percentage
          </button>
          <button 
            @click="spreadUnit = 'bps'" 
            :class="['funding-toggle-btn', { active: spreadUnit === 'bps' }]"
          >
            Basis Points
          </button>
        </div>
      </div>
    </div>

    <!-- Dashboard Content -->
    <div class="funding-dashboard">
      <!-- Loading State -->
      <div v-if="isLoading && !tokens.length" class="funding-loading-container">
        <div class="funding-loading-spinner"></div>
        <p>Loading funding rates...</p>
      </div>

      <!-- Error State -->
      <div v-else-if="error" class="funding-error-container">
        <p class="funding-error-message">{{ error }}</p>
        <button @click="fetchData" class="funding-retry-button">Retry</button>
      </div>

      <!-- Data Display -->
      <div v-else class="funding-dashboard-card">
        <!-- Table View -->
        <FundingTable 
          v-if="viewMode === 'table'"
          :tokens="sortedTokens"
          :exchanges="exchanges"
          :display-mode="displayMode"
          :spread-unit="spreadUnit"
          :sort-column="sortColumn"
          :sort-direction="sortDirection"
          @sort="handleSort"
        />
        
        <!-- Matrix View -->
        <FundingMatrix 
          v-else-if="viewMode === 'matrix'"
          :tokens="sortedTokens"
          :exchanges="exchanges"
          :displayMode="displayMode"     
          :spreadUnit="spreadUnit"        
          :sortColumn="sortColumn"       
          :sortDirection="sortDirection" 
          @sort="handleSort"
        />

        <!-- Footer Information -->
        <div class="funding-last-updated">
          <span v-if="lastUpdated">
            Last updated: {{ formatTimestamp(lastUpdated) }}
          </span>
          <span v-if="isLoading" class="funding-updating-indicator">
            Updating...
          </span>
          <span class="funding-rate-info">
            {{ displayMode === 'annualized' ? '(3x daily payments)' : '8-hour funding rate' }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import FundingTable from './FundingTable.vue'
import FundingMatrix from './FundingMatrix.vue'
import { fetchFundingRates } from '@/api/funding'
import { handleApiError } from '@/api/error'         
import { formatTimestamp } from '../../utils/formatters'
import { sumOpenInterest, calculateArbSpread, findBestArbOpportunity } from '../../utils/calculations'
import { REFRESH_INTERVAL } from '../../utils/constants'
import type { TokenRow, DisplayMode, SpreadUnit, SortDirection } from '../../utils/types'

// State
const tokens = ref<TokenRow[]>([])
const lastUpdated = ref<string>('')
const exchanges = ref<string[]>([])
const isLoading = ref(false)
const error = ref<string | null>(null)

// Display options
const viewMode = ref<'table' | 'matrix'>('table')
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