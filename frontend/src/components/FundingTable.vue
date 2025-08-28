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
            <div class="arb-combo-display" v-if="findBestArbOpportunity(row.exchanges)">
              <div class="arb-strategy">
                {{ formatArbOpportunity(findBestArbOpportunity(row.exchanges)) }}
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
                :class="getRateClass(row.exchanges[ex].funding_rate, row.exchanges, ex)"
              >
                {{ formatRate(row.exchanges[ex].funding_rate, displayMode) }}
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
import { computed } from 'vue'
import { formatNumber, formatRate, formatSpread, formatArbOpportunity } from '../utils/formatters'
import { sumOpenInterest, calculateArbSpread, findBestArbOpportunity } from '../utils/calculations'
import { getArbClass, getArbOpportunityClass, getRateClass } from '../utils/styles'
import type { TokenRow, DisplayMode, SpreadUnit, SortDirection, ArbOpportunity } from '../utils/types'

interface Props {
  tokens: TokenRow[]
  exchanges: string[]
  displayMode: DisplayMode
  spreadUnit: SpreadUnit
  sortColumn: string
  sortDirection: SortDirection
}

const props = defineProps<Props>()

const emit = defineEmits<{
  sort: [column: string]
}>()

// Helper function to determine row class
function getRowClass(row: TokenRow): string {
  const arb = findBestArbOpportunity(row.exchanges)
  if (!arb) return ''
  
  const bothPositive = arb.longRate > 0 && arb.shortRate > 0
  const bothNegative = arb.longRate < 0 && arb.shortRate < 0
  
  // Highlight rows with extreme same-sign spreads
  if ((bothPositive || bothNegative) && arb.spread >= 50) {
    return 'row-extreme-capture'
  }
  if ((bothPositive || bothNegative) && arb.spread >= 25) {
    return 'row-high-capture'
  }
  if (arb.spread >= 25) {
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

// Get capture info text
function getCaptureInfo(arb: ArbOpportunity | null): string {
  if (!arb) return ''
  const bothPositive = arb.longRate > 0 && arb.shortRate > 0
  
  if (bothPositive) {
    return `Both positive, ${arb.spread.toFixed(0)}bps spread`
  } else {
    return `Both negative, ${arb.spread.toFixed(0)}bps spread`
  }
}
</script>
