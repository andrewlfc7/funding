import { ref, computed } from 'vue'
import { 
  fetchLeadersLaggards,
  type LeadersLaggardsResponse,
  type LeadersLaggardsParams,
  type CoinRanking,
  type VolumeSpike,
  type DecorrelatedAsset
} from '@/api/zscore/leadersLaggards'

export function useLeadersLaggards() {
  const loading = ref(false)
  const error = ref<string | null>(null)
  const data = ref<LeadersLaggardsResponse | null>(null)

  // Direct access to API data
  const leaders = computed(() => data.value?.leaders || [])
  const laggards = computed(() => data.value?.laggards || [])
  const volumeSpikes = computed(() => data.value?.volumeSpikes || [])
  const decorrelatedAssets = computed(() => data.value?.decorrelated || [])
  const leadLagMatrix = computed(() => data.value?.leadLagMatrix || null)

  // Computed properties
  const topCoins = computed(() => {
    if (!data.value) return []
    // Get unique coins from leaders
    const coins = new Set<string>()
    data.value.leaders.forEach(l => coins.add(l.symbol))
    return Array.from(coins).slice(0, 30)
  })

  const marketStats = computed(() => {
    if (!data.value) {
      return {
        totalCoins: 0,
        meanZScore: 0,
        stdZScore: 1,
        bullishCount: 0,
        bearishCount: 0,
        neutralCount: 0,
        marketBreadth: 0
      }
    }

    const allCoins = [...data.value.leaders]
    const bullish = allCoins.filter(r => r.zscore > 1).length
    const bearish = allCoins.filter(r => r.zscore < -1).length
    const neutral = allCoins.length - bullish - bearish

    const zscores = allCoins.map(r => r.zscore)
    const mean = zscores.reduce((a, b) => a + b, 0) / zscores.length
    const variance = zscores.reduce((sum, z) => sum + Math.pow(z - mean, 2), 0) / zscores.length
    const std = Math.sqrt(variance)
    
    return {
      totalCoins: allCoins.length,
      meanZScore: mean,
      stdZScore: std,
      bullishCount: bullish,
      bearishCount: bearish,
      neutralCount: neutral,
      marketBreadth: (mean / std) 
    }
  })

  // Get decorrelation status
  function getDecorrelationStatus(correlation: number): 'critical' | 'alert' | 'warning' | 'normal' {
    const abs = Math.abs(correlation)
    if (abs < 0.1) return 'critical'
    if (abs < 0.3) return 'alert'
    if (abs < 0.5) return 'warning'
    return 'normal'
  }

  // Fetch data
  async function fetchData(params: LeadersLaggardsParams) {
    loading.value = true
    error.value = null

    try {
      data.value = await fetchLeadersLaggards(params)
    } catch (e) {
      error.value = e instanceof Error ? e.message : 'Failed to fetch data'
      console.error('Leaders/Laggards error:', e)
    } finally {
      loading.value = false
    }
  }

  // Format returns for display
  function formatReturns(returns: number): string {
    const percentage = returns * 100
    const sign = percentage > 0 ? '+' : ''
    return `${sign}${percentage.toFixed(2)}%`
  }

  // Format volume
  function formatVolume(volume: number): string {
    if (volume >= 1e9) return `${(volume / 1e9).toFixed(2)}B`
    if (volume >= 1e6) return `${(volume / 1e6).toFixed(2)}M`
    if (volume >= 1e3) return `${(volume / 1e3).toFixed(2)}K`
    return volume.toFixed(2)
  }

  // Get lead-lag data for a specific pair
  function getLeadLagData(coin1: string, coin2: string) {
    if (!leadLagMatrix.value) return null

    const { coins, lags, matrix } = leadLagMatrix.value
    const coin1Index = coins.indexOf(coin1)
    const coin2Index = coins.indexOf(coin2)

    if (coin1Index === -1 || coin2Index === -1) return null

    // Extract correlation values for the pair at different lags
    const correlations = lags.map((_, lagIndex) => {
      return matrix[lagIndex][coin1Index][coin2Index]
    })

    // Find optimal lag (highest absolute correlation)
    let maxCorr = 0
    let optimalLag = 0
    correlations.forEach((corr, index) => {
      if (Math.abs(corr) > Math.abs(maxCorr)) {
        maxCorr = corr
        optimalLag = lags[index]
      }
    })

    return {
      lags,
      correlations,
      optimalLag,
      maxCorrelation: maxCorr
    }
  }

  return {
    loading,
    error,
    data,
    leaders,
    laggards,
    volumeSpikes,
    decorrelatedAssets,
    leadLagMatrix,
    topCoins,
    marketStats,
    fetchData,
    formatReturns,
    formatVolume,
    getLeadLagData,
    getDecorrelationStatus
  }
}