// src/composables/useMetaData.ts
import { ref, computed } from 'vue'
import { fetchMetaData, type MetaData } from '@/api/meta'
import { SELECTED_QUOTE } from '@/utils/constants'

export function useMetaData() {
  const metaData = ref<MetaData>({ exchanges: [], coins: [] })
  const loading = ref(false)
  const error = ref<string | null>(null)
  
  const exchanges = computed(() => metaData.value.exchanges)
  const coins = computed(() => metaData.value.coins)
  
  // Default values
  const defaultExchange = computed(() => exchanges.value[0] || 'Binance')
  const defaultCoin = computed(() => coins.value.includes('BTC') ? 'BTC' : coins.value[0] || 'BTC')
  
  async function loadMetaData(marketType: string = 'spot', quote: string = SELECTED_QUOTE) {
    loading.value = true
    error.value = null
    
    try {
      const data = await fetchMetaData(marketType, quote)
      metaData.value = data
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to load meta data'
      console.error('Meta data fetch error:', e)
      metaData.value = {
        exchanges: ['Binance'],
        coins: ['BTC', 'ETH', 'SOL']
      }
    } finally {
      loading.value = false
    }
  }
  
  return {
    exchanges,
    coins,
    defaultExchange,
    defaultCoin,
    loading,
    error,
    loadMetaData
  }
}