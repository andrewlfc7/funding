
<template>
  <div class="table-container">
    <table class="funding-table">
      <thead>
        <tr>
          <th @click="$emit('sort', 'token')" class="sortable">
            Symbol
            <span class="sort-indicator">{{ getSortIndicator('token') }}</span>
          </th>
          <th @click="$emit('sort', 'openInterest')" class="sortable">
            Total OI
            <span class="sort-indicator">{{ getSortIndicator('openInterest') }}</span>
          </th>
          <th @click="$emit('sort', 'arbSpread')" class="sortable center">
            Best Spread
            <span class="sort-indicator">{{ getSortIndicator('arbSpread') }}</span>
          </th>
          <th @click="$emit('sort', 'arbCombo')" class="sortable center arb-combo-header">
            Arb Pair
            <span class="sort-indicator">{{ getSortIndicator('arbCombo') }}</span>
          </th>
          <th @click="$emit('sort', 'arbOI')" class="sortable center">
            Pair OI
            <span class="sort-indicator">{{ getSortIndicator('arbOI') }}</span>
          </th>
          <th 
            v-for="ex in exchanges" 
            :key="ex" 
            @click="$emit('sort', `exchange_${ex}`)"
            class="sortable center exchange-header"
          >
            {{ ex }}
            <span class="sort-indicator">{{ getSortIndicator(`exchange_${ex}`) }}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="row in tokens" :key="row.token" :class="getRowClass(row)">
          <td class="token-cell">{{ row.token }}</td>
          <td class="open-interest">
            ${{ formatNumber(sumOpenInterest(row.exchanges)) }}
          </td>
          <td class="center arb-spread" :class="getArbClass(calculateArbSpread(row.exchanges))">
            {{ formatArbSpread(calculateArbSpread(row.exchanges)) }}
          </td>
          <td class="center arb-combo" :class="getArbOpportunityClass(findBestArbOpportunity(row.exchanges))">
            <div 
              class="arb-combo-display" 
              v-if="findBestArbOpportunity(row.exchanges)"
              :class="getArbComboDisplayClass(findBestArbOpportunity(row.exchanges))"
            >
              <div class="arb-strategy">
                <span class="long-exchange">{{ findBestArbOpportunity(row.exchanges)?.longExchange }}</span>
                <span>→</span>
                <span class="short-exchange">{{ findBestArbOpportunity(row.exchanges)?.shortExchange }}</span>
              </div>
              <div v-if="showCaptureInfo(findBestArbOpportunity(row.exchanges))" class="arb-capture-info">
                {{ getCaptureInfo(findBestArbOpportunity(row.exchanges)) }}
              </div>
            </div>
            <span v-else>–</span>
          </td>
          <td class="center arb-oi">
            <span v-if="findBestArbOpportunity(row.exchanges)">
              ${{ formatNumber(findBestArbOpportunity(row.exchanges)?.combinedOI || 0) }}
            </span>
            <span v-else>–</span>
          </td>
          <td v-for="ex in exchanges" :key="ex" class="center exchange-cell">
            <div v-if="row.exchanges[ex]" class="exchange-data">
              <span
                class="funding-rate"
                :class="getFundingRateClass(row.exchanges[ex].funding_rate, row.exchanges, ex)"
                :title="getFundingRateTooltip(row.exchanges[ex], ex)"
              >
                {{ formatRate(row.exchanges[ex].funding_rate, displayMode, spreadUnit) }}
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
</template>

<script setup lang="ts">
import { computed, withDefaults } from 'vue'
import { formatNumber, formatRate, formatSpread } from '../../utils/formatters'
import { sumOpenInterest, calculateArbSpread, findBestArbOpportunity } from '../../utils/calculations'
import { getArbClass, getArbOpportunityClass } from '../../utils/styles'
import type { TokenRow, DisplayMode, SpreadUnit, SortDirection, ArbOpportunity, ExchangeData } from '../../utils/types'

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

// Enhanced funding rate classification with new color system
function getFundingRateClass(rate: number, exchanges: Record<string, ExchangeData>, currentExchange: string): string {
  const rates = Object.values(exchanges).map(ex => ex.funding_rate).sort((a, b) => b - a)
  
  if (rates.length < 2) {
    // Single exchange or no comparison - use basic color coding
    if (rate > 0) return 'rate-positive'
    if (rate < 0) return 'rate-negative'
    return 'rate-neutral'
  }
  
  const maxRate = rates[0]
  const minRate = rates[rates.length - 1]
  const spreadBps = (maxRate - minRate) * 10000
  const rateBps = Math.abs(rate) * 10000
  
  // Check for extreme conditions
  const allPositive = minRate > 0
  const allNegative = maxRate < 0
  
  // Extreme opportunities when spread is large (25+ bps) and all same sign
  if ((allPositive || allNegative) && spreadBps >= 100) {
    if (rate === maxRate) return 'rate-extreme-short'
    if (rate === minRate) return 'rate-extreme-long'
  }
  
  // Very high magnitude individual rates (50+ bps)
  if (rateBps >= 100) {
    return rate > 0 ? 'rate-extreme-short' : 'rate-extreme-long'
  }
  
  // Regular arbitrage opportunities
  if (spreadBps >= 15) { // 15+ bps spread threshold
    if (rate === maxRate) return 'rate-short-candidate'
    if (rate === minRate) return 'rate-long-candidate'
  }
  
  if (spreadBps >= 1) { // 8+ bps spread threshold  
    if (rate === maxRate) return 'rate-short-opportunity'
    if (rate === minRate) return 'rate-long-opportunity'
  }
  
  // Basic color coding based on sign
  if (rate > 0) return 'rate-positive'
  if (rate < 0) return 'rate-negative'
  
  return 'rate-neutral'
}

// Helper function to determine row class
function getRowClass(row: TokenRow): string {
  const arb = findBestArbOpportunity(row.exchanges)
  if (!arb) return ''
  
  const bothPositive = arb.longRate > 0 && arb.shortRate > 0
  const bothNegative = arb.longRate < 0 && arb.shortRate < 0
  
  if ((bothPositive || bothNegative) && arb.spread >= 100) {
    return 'row-extreme-capture'
  }
  if ((bothPositive || bothNegative) && arb.spread >= 100) {
    return 'row-high-capture'
  }
  if (arb.spread >= 100) {
    return 'row-high-spread'
  }
  
  return ''
}

// Sort indicator helper
function getSortIndicator(column: string): string {
  if (props.sortColumn !== column) return '↕'
  return props.sortDirection === 'asc' ? '↑' : '↓'
}

// Format arbitrage spread with units
function formatArbSpread(spreadBps: number | null): string {
  if (spreadBps === null) return '–'
  return formatSpread(spreadBps, props.spreadUnit)
}

// Check if we should show capture info
function showCaptureInfo(arb: ArbOpportunity | null): boolean {
  if (!arb) return false
  const bothPositive = arb.longRate > 0 && arb.shortRate > 0
  const bothNegative = arb.longRate < 0 && arb.shortRate < 0
  return (bothPositive || bothNegative) && arb.spread >= 25  // 25+ bps threshold
}

function getCaptureInfo(arb: ArbOpportunity | null): string {
  if (!arb) return ''
  const bothPositive = arb.longRate > 0 && arb.shortRate > 0
  
  const spreadText = props.spreadUnit === 'percentage' 
    ? `${(arb.spread / 100).toFixed(2)}%` 
    : `${arb.spread.toFixed(0)}bps`
  
  if (bothPositive) {
    return `Both positive, ${spreadText} spread`
  } else {
    return `Both negative, ${spreadText} spread`
  }
}

function getArbComboDisplayClass(arb: ArbOpportunity | null): string {
  if (!arb) return '';
  const bothPositive = arb.longRate > 0 && arb.shortRate > 0;
  const bothNegative = arb.longRate < 0 && arb.shortRate < 0;
  
  if ((bothPositive || bothNegative) && arb.spread >= 25) {
    return 'arb-capture-highlight'; // A descriptive class for styling these special cases
  }
  
  return '';
}

// Enhanced tooltip for funding rates
function getFundingRateTooltip(exchangeData: ExchangeData, exchange: string): string {
  const rate = (exchangeData.funding_rate * 100).toFixed(4)
  const annualized = (exchangeData.funding_rate * 365 * 3 * 100).toFixed(2)
  const oi = formatNumber(exchangeData.open_interest)
  
  let interpretation = ''
  if (exchangeData.funding_rate > 0) {
    interpretation = 'Longs pay shorts'
  } else if (exchangeData.funding_rate < 0) {
    interpretation = 'Shorts pay longs'
  } else {
    interpretation = 'Neutral funding'
  }
  
  return `${exchange}: ${rate}% (8h) | ${annualized}% (annualized) | ${interpretation} | OI: $${oi}`
}
</script>