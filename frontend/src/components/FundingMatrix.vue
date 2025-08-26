<template>
  <div>
    <!-- Page Header -->
    <header class="page-header">
      <h1 class="page-title">Funding Rates Arbitrage</h1>
      <p class="page-subtitle">Cross-Dex</p>
    </header>

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
        <div class="table-container">
          <table class="funding-table">
            <thead>
              <tr>
                <th @click="sortBy('token')" class="sortable">
                  Symbol
                  <span class="sort-indicator">{{ getSortIndicator('token') }}</span>
                </th>
                <th @click="sortBy('openInterest')" class="sortable">
                  Open Interest
                  <span class="sort-indicator">{{ getSortIndicator('openInterest') }}</span>
                </th>
                <th @click="sortBy('arbSpread')" class="sortable center">
                  Arb Spread
                  <span class="sort-indicator">{{ getSortIndicator('arbSpread') }}</span>
                </th>
                <th 
                  v-for="ex in exchanges" 
                  :key="ex" 
                  @click="sortBy(`exchange_${ex}`)"
                  class="sortable center"
                >
                  {{ ex }}
                  <span class="sort-indicator">{{ getSortIndicator(`exchange_${ex}`) }}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in sortedTokens" :key="row.token">
                <td class="token-cell">{{ row.token }}</td>
                <td class="open-interest">
                  ${{ formatNumber(sumOpenInterest(row.exchanges)) }}
                </td>
                <td class="center arb-spread" :class="getArbClass(calculateArbSpread(row.exchanges))">
                  {{ formatArbSpread(calculateArbSpread(row.exchanges)) }}
                </td>
                <td v-for="ex in exchanges" :key="ex" class="center">
                  <span
                    v-if="row.exchanges[ex]"
                    :class="getRateClass(row.exchanges[ex].funding_rate)"
                  >
                    {{ (row.exchanges[ex].funding_rate * 100).toFixed(3) }}%
                  </span>
                  <span v-else class="rate-missing">–</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        
        <div class="last-updated">
          <span v-if="lastUpdated">
            Last updated: {{ formatTimestamp(lastUpdated) }}
          </span>
          <span v-if="isLoading" class="updating-indicator">
            Updating...
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import axios from 'axios'

interface ExchangeData {
  funding_rate: number
  open_interest: number
}

interface TokenRow {
  token: string
  exchanges: Record<string, ExchangeData>
}

interface ApiResponse {
  last_updated: string
  tokens: TokenRow[]
}

// API Configuration
const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:8080'
const API_ENDPOINT = '/api/funding-rates'
const REFRESH_INTERVAL = 30000 // 30 seconds

// Reactive data
const tokens = ref<TokenRow[]>([])
const lastUpdated = ref<string>('')
const exchanges = ref<string[]>([])
const isLoading = ref(false)
const error = ref<string | null>(null)

// Sorting state
const sortColumn = ref<string>('')
const sortDirection = ref<'asc' | 'desc'>('desc')

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

// Fetch data from API
async function fetchData() {
  try {
    // Don't show loading on refresh if we already have data
    if (!tokens.value.length) {
      isLoading.value = true
    }
    error.value = null
    
    const response = await axios.get<ApiResponse>(`${API_BASE_URL}${API_ENDPOINT}`)
    
    tokens.value = response.data.tokens
    lastUpdated.value = response.data.last_updated
    
    // Extract unique exchanges
    const allExchanges = new Set<string>()
    tokens.value.forEach(t => {
      Object.keys(t.exchanges).forEach(ex => allExchanges.add(ex))
    })
    exchanges.value = Array.from(allExchanges).sort()
    
  } catch (err) {
    console.error('Failed to fetch funding rates:', err)
    error.value = err instanceof Error ? err.message : 'Failed to fetch funding rates'
  } finally {
    isLoading.value = false
  }
}

// Lifecycle hooks
onMounted(() => {
  fetchData()
  // Set up auto-refresh
  refreshInterval = window.setInterval(fetchData, REFRESH_INTERVAL)
})

onUnmounted(() => {
  if (refreshInterval) {
    clearInterval(refreshInterval)
  }
})

// Sorting functions
function sortBy(column: string) {
  if (sortColumn.value === column) {
    sortDirection.value = sortDirection.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortColumn.value = column
    sortDirection.value = 'desc'
  }
}

function getSortIndicator(column: string): string {
  if (sortColumn.value !== column) return '↕'
  return sortDirection.value === 'asc' ? '↑' : '↓'
}

// Utility functions
function formatNumber(value: number): string {
  if (value >= 1e9) return (value / 1e9).toFixed(2) + 'B'
  if (value >= 1e6) return (value / 1e6).toFixed(2) + 'M'
  if (value >= 1e3) return (value / 1e3).toFixed(2) + 'K'
  return value.toFixed(0)
}

function sumOpenInterest(exs: Record<string, ExchangeData>): number {
  return Object.values(exs).reduce((s, v) => s + v.open_interest, 0)
}

function calculateArbSpread(exchanges: Record<string, ExchangeData>): number | null {
  const rates = Object.values(exchanges).map(ex => ex.funding_rate)
  
  if (rates.length < 2) return null
  
  const maxRate = Math.max(...rates)
  const minRate = Math.min(...rates)
  
  return (maxRate - minRate) * 10000
}

function formatArbSpread(spreadBps: number | null): string {
  if (spreadBps === null) return '–'
  return spreadBps.toFixed(0) + ' bps'
}

function getArbClass(spreadBps: number | null): string {
  if (spreadBps === null) return ''
  if (spreadBps >= 10) return 'arb-high'
  if (spreadBps >= 5) return 'arb-medium'
  return 'arb-low'
}

function getRateClass(rate: number): string {
  if (rate > 0) return 'rate-positive'
  if (rate < 0) return 'rate-negative'
  return 'rate-neutral'
}

function formatTimestamp(ts: string): string {
  return new Date(ts).toLocaleString()
}
</script>