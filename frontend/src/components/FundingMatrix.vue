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
                  Total OI
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
                  class="sortable center exchange-header"
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
                <td v-for="ex in exchanges" :key="ex" class="center exchange-cell">
                  <div v-if="row.exchanges[ex]" class="exchange-data">
                    <span
                      class="funding-rate"
                      :class="getRateClass(row.exchanges[ex].funding_rate)"
                    >
                      {{ (row.exchanges[ex].funding_rate * 100).toFixed(3) }}%
                    </span>
                    <span class="open-interest-per-exchange">
                      ${{ formatNumber(row.exchanges[ex].open_interest) }}
                    </span>
                  </div>
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

const API_BASE_URL = import.meta.env.VITE_API_URL
const API_ENDPOINT = import.meta.env.VITE_API_ENDPOINT
const REFRESH_INTERVAL = parseInt(import.meta.env.VITE_REFRESH_INTERVAL)

if (!API_BASE_URL) {
  throw new Error('VITE_API_URL environment variable is required')
}
if (!API_ENDPOINT) {
  throw new Error('VITE_API_ENDPOINT environment variable is required')
}
if (!REFRESH_INTERVAL || isNaN(REFRESH_INTERVAL)) {
  throw new Error('VITE_REFRESH_INTERVAL environment variable is required and must be a number')
}

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

async function fetchData() {
  try {
    if (!tokens.value.length) {
      isLoading.value = true
    }
    error.value = null
    
    console.log('Fetching data from:', `${API_BASE_URL}${API_ENDPOINT}`)
    
    const response = await axios.get<ApiResponse>(`${API_BASE_URL}${API_ENDPOINT}`)
    
    // Log the raw response
    console.log('Raw API Response:', response.data)
    
    tokens.value = response.data.tokens
    lastUpdated.value = response.data.last_updated
    
    // Log if tokens array is empty
    if (!tokens.value || tokens.value.length === 0) {
      console.warn('No tokens received from API - tokens array is empty')
    } else {
      console.log(`Received ${tokens.value.length} tokens`)
      
      // Log first few tokens as sample
      console.log('Sample tokens (first 3):', tokens.value.slice(0, 3))
      
      // Log token details
      tokens.value.slice(0, 3).forEach((token, index) => {
        console.log(`Token ${index + 1}: ${token.token}`)
        console.log('  Exchanges:', Object.keys(token.exchanges))
        console.log('  Exchange Data:', token.exchanges)
      })
    }
    
    // Extract and log exchanges
    const allExchanges = new Set<string>()
    tokens.value.forEach(t => {
      Object.keys(t.exchanges).forEach(ex => allExchanges.add(ex))
    })
    exchanges.value = Array.from(allExchanges).sort()
    
    console.log('Detected exchanges:', exchanges.value)
    console.log('Last updated:', lastUpdated.value)
    
    // Log summary statistics
    if (tokens.value.length > 0) {
      const totalOI = tokens.value.reduce((sum, token) => 
        sum + sumOpenInterest(token.exchanges), 0
      )
      console.log('Total Open Interest across all tokens:', formatNumber(totalOI))
      
      // Find highest arb opportunities
      const arbOpportunities = tokens.value
        .map(token => ({
          token: token.token,
          spread: calculateArbSpread(token.exchanges)
        }))
        .filter(item => item.spread !== null)
        .sort((a, b) => (b.spread ?? 0) - (a.spread ?? 0))
        .slice(0, 5)
      
      console.log('Top 5 Arbitrage Opportunities:', arbOpportunities)
    }
    
  } catch (err) {
    console.error('Failed to fetch funding rates:', err)
    
    // Log more detailed error info
    if (axios.isAxiosError(err)) {
      console.error('Axios error details:', {
        message: err.message,
        status: err.response?.status,
        statusText: err.response?.statusText,
        data: err.response?.data,
        url: err.config?.url
      })
    }
    
    error.value = err instanceof Error ? err.message : 'Failed to fetch funding rates'
  } finally {
    isLoading.value = false
    console.log('Data fetch completed')
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