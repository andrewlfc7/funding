<template>
  <div class="funding-matrix-container">
    <!-- Matrix Header -->
    <div class="funding-matrix-header">
      <h3 class="funding-matrix-title">Funding Rates Matrix</h3>
      <div class="funding-matrix-legend">
        <div class="legend-item">
          <div class="legend-color legend-positive"></div>
          <span>Longs Pay Shorts (Red)</span>
        </div>
        <div class="legend-item">
          <div class="legend-color legend-negative"></div>
          <span>Shorts Pay Longs (Green)</span>
        </div>
      </div>
    </div>

    <!-- Scrollable Matrix -->
    <div class="funding-matrix-wrapper" ref="matrixWrapper">
      <table class="funding-matrix-table">
        <thead>
          <tr>
            <th class="matrix-symbol-header">Symbol</th>
            <th 
              v-for="exchange in exchanges" 
              :key="exchange" 
              class="matrix-exchange-header"
              @click="handleExchangeSort(exchange)"
            >
              {{ exchange }}
              <span class="sort-indicator">{{ getExchangeSortIndicator(exchange) }}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          <tr 
            v-for="token in sortedTokens" 
            :key="token.token"
            class="matrix-row"
            :class="getMatrixRowClass(token)"
          >
            <td class="matrix-symbol-cell">
              <div class="symbol-info">
                <span class="symbol-name">{{ token.token }}</span>
                <span class="symbol-oi">${{ formatOI(sumOpenInterest(token.exchanges)) }}</span>
              </div>
            </td>
            <td 
              v-for="exchange in exchanges" 
              :key="exchange" 
              class="matrix-rate-cell"
            >
              <div 
                v-if="token.exchanges[exchange]" 
                class="matrix-rate-data"
              >
                <div 
                  class="matrix-funding-rate"
                  :class="getMatrixRateClass(token.exchanges[exchange].funding_rate)"
                  :title="getRateTooltip(token.exchanges[exchange], exchange)"
                >
                  {{ formatRate(token.exchanges[exchange].funding_rate, displayMode, spreadUnit) }}
                </div>
                <div class="matrix-oi">
                  ${{ formatOI(token.exchanges[exchange].open_interest) }}
                </div>
              </div>
              <div v-else class="matrix-rate-missing">
                <span>—</span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Scroll Indicators -->
    <div class="scroll-indicators">
      <button 
        @click="scrollLeft" 
        class="scroll-btn scroll-left"
        :disabled="!canScrollLeft"
      >
        ‹
      </button>
      <div class="scroll-progress">
        <div 
          class="scroll-progress-bar" 
          :style="{ width: scrollProgress + '%' }"
        ></div>
      </div>
      <button 
        @click="scrollRight" 
        class="scroll-btn scroll-right"
        :disabled="!canScrollRight"
      >
        ›
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, withDefaults } from 'vue'
import { formatNumber, formatRate } from '../../utils/formatters'
import { sumOpenInterest } from '../../utils/calculations'
import type { TokenRow, DisplayMode, SpreadUnit, SortDirection, ExchangeData } from '../../utils/types'

interface Props {
  tokens: TokenRow[]
  exchanges: string[]
  displayMode: DisplayMode
  spreadUnit: SpreadUnit
  sortColumn: string
  sortDirection: SortDirection
}

const props = withDefaults(defineProps<Props>(), {
  tokens: () => [],
  exchanges: () => [],
  displayMode: 'rate',
  spreadUnit: 'percentage',
  sortColumn: '',
  sortDirection: 'desc'
})
const emit = defineEmits<{
  sort: [column: string]
}>()

// Matrix scrolling refs
const matrixWrapper = ref<HTMLElement>()
const canScrollLeft = ref(false)
const canScrollRight = ref(false)
const scrollProgress = ref(0)

// Sorting for matrix
const matrixSortColumn = ref<string>('')
const matrixSortDirection = ref<SortDirection>('desc')

// Computed sorted tokens for matrix - WITH NULL CHECK
const sortedTokens = computed(() => {
  // Add null/undefined check here
  if (!props.tokens || !Array.isArray(props.tokens)) {
    return []
  }
  
  const tokensCopy = [...props.tokens]
  
  if (!matrixSortColumn.value) return tokensCopy
  
  return tokensCopy.sort((a, b) => {
    let aValue: any
    let bValue: any
    
    if (matrixSortColumn.value === 'token') {
      aValue = a.token
      bValue = b.token
    } else if (matrixSortColumn.value === 'totalOI') {
      aValue = sumOpenInterest(a.exchanges)
      bValue = sumOpenInterest(b.exchanges)
    } else if (matrixSortColumn.value.startsWith('exchange_')) {
      const exchange = matrixSortColumn.value.replace('exchange_', '')
      aValue = a.exchanges[exchange]?.funding_rate ?? -999
      bValue = b.exchanges[exchange]?.funding_rate ?? -999
    }
    
    if (aValue < bValue) return matrixSortDirection.value === 'asc' ? -1 : 1
    if (aValue > bValue) return matrixSortDirection.value === 'asc' ? 1 : -1
    return 0
  })
})

// Matrix-specific rate classification
function getMatrixRateClass(rate: number): string {
  const absRate = Math.abs(rate)
  const rateBps = absRate * 10000

  // Color based on sign: positive = red, negative = green
  let baseClass = rate > 0 ? 'matrix-rate-positive' : 'matrix-rate-negative'
  
  // Add magnitude classes
  if (rateBps >= 50) return `${baseClass} matrix-rate-extreme`
  if (rateBps >= 25) return `${baseClass} matrix-rate-very-high`
  if (rateBps >= 15) return `${baseClass} matrix-rate-high`
  if (rateBps >= 8) return `${baseClass} matrix-rate-medium`
  if (rateBps >= 3) return `${baseClass} matrix-rate-low`
  
  return baseClass
}

function getMatrixRowClass(token: TokenRow): string {
  const rates = Object.values(token.exchanges).map(ex => ex.funding_rate)
  const maxRate = Math.max(...rates)
  const minRate = Math.min(...rates)
  const spread = (maxRate - minRate) * 10000

  if (spread >= 50) return 'matrix-row-extreme'
  if (spread >= 25) return 'matrix-row-high'
  return ''
}

function handleExchangeSort(exchange: string) {
  const column = `exchange_${exchange}`
  if (matrixSortColumn.value === column) {
    matrixSortDirection.value = matrixSortDirection.value === 'asc' ? 'desc' : 'asc'
  } else {
    matrixSortColumn.value = column
    matrixSortDirection.value = 'desc'
  }
}

function getExchangeSortIndicator(exchange: string): string {
  const column = `exchange_${exchange}`
  if (matrixSortColumn.value !== column) return '↕'
  return matrixSortDirection.value === 'asc' ? '↑' : '↓'
}

function getRateTooltip(exchangeData: ExchangeData, exchange: string): string {
  const rate = (exchangeData.funding_rate * 100).toFixed(4)
  const oi = formatOI(exchangeData.open_interest)
  const annualized = (exchangeData.funding_rate * 365 * 3 * 100).toFixed(2)
  return `${exchange}: ${rate}% (8h) | ${annualized}% (annualized) | OI: $${oi}`
}

function formatOI(value: number): string {
  return formatNumber(value)
}

// Scrolling functionality
function updateScrollState() {
  if (!matrixWrapper.value) return
  
  const element = matrixWrapper.value
  canScrollLeft.value = element.scrollLeft > 0
  canScrollRight.value = element.scrollLeft < (element.scrollWidth - element.clientWidth - 1)
  
  // Update progress
  const maxScroll = element.scrollWidth - element.clientWidth
  scrollProgress.value = maxScroll > 0 ? (element.scrollLeft / maxScroll) * 100 : 0
}

function scrollLeft() {
  if (!matrixWrapper.value) return
  matrixWrapper.value.scrollBy({ left: -300, behavior: 'smooth' })
}

function scrollRight() {
  if (!matrixWrapper.value) return
  matrixWrapper.value.scrollBy({ left: 300, behavior: 'smooth' })
}

// Handle scroll events
function onScroll() {
  updateScrollState()
}

onMounted(() => {
  nextTick(() => {
    if (matrixWrapper.value) {
      matrixWrapper.value.addEventListener('scroll', onScroll)
      updateScrollState()
    }
  })
})

onUnmounted(() => {
  if (matrixWrapper.value) {
    matrixWrapper.value.removeEventListener('scroll', onScroll)
  }
})
</script>