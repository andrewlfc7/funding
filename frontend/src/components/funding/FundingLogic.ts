import { ref, computed, onMounted, onUnmounted } from 'vue'
import { fetchFundingRates } from '@/api/funding'

import { handleApiError } from '@/api/error'            

import { formatTimestamp } from '../../utils/formatters'
import { sumOpenInterest, calculateArbSpread, findBestArbOpportunity } from '../../utils/calculations'
import { REFRESH_INTERVAL } from '../../utils/constants'
import type { TokenRow, DisplayMode, SpreadUnit, SortDirection } from '../../utils/types'

// State
export const tokens = ref<TokenRow[]>([])
export const lastUpdated = ref<string>('')
export const exchanges = ref<string[]>([])
export const isLoading = ref(false)
export const error = ref<string | null>(null)

export const displayMode = ref<DisplayMode>('rate')
export const spreadUnit = ref<SpreadUnit>('percentage')

export const sortColumn = ref<string>('')
export const sortDirection = ref<SortDirection>('desc')

let refreshInterval: number | undefined

export const sortedTokens = computed(() => {
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

export async function fetchData() {
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

export function handleSort(column: string) {
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
