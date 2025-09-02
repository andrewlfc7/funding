// src/api/zscore/helpers.ts
import type { ZScoreDataPoint, VolatilityDataPoint } from './types'

export function groupDataBySymbol<T extends { symbol: string }>(data: T[]): Map<string, T[]> {
  const grouped = new Map<string, T[]>()
  for (const point of data) {
    const list = grouped.get(point.symbol)
    if (list) list.push(point)
    else grouped.set(point.symbol, [point])
  }
  return grouped
}

export function getLatestBySymbol<T extends { symbol: string; timestamp: number }>(data: T[]): Map<string, T> {
  const latest = new Map<string, T>()
  for (const point of data) {
    const prev = latest.get(point.symbol)
    if (!prev || point.timestamp > prev.timestamp) latest.set(point.symbol, point)
  }
  return latest
}

export function transformZScoreToTimeSeries(data: ZScoreDataPoint[]) {
  return data.map((d) => ({
    timestamp: d.timestamp * 1000, // sec → ms
    value: d.zscore,
    price: d.price,
    volume: d.volume,
    returns1h: d.returns1h,
    returns1d: d.returns1d,
    symbol: d.symbol,
  }))
}
export const zrowToSeries = transformZScoreToTimeSeries

export function transformVolatilityToTimeSeries(data: VolatilityDataPoint[]) {
  return data.map((d) => ({
    timestamp: d.timestamp * 1000,
    volatility: d.volatility * 100,
    volatilityZScore: d.volatilityZScore,
    returns: d.returns * 100,
    range: d.range * 100,
    symbol: d.symbol,
  }))
}
export const volrowToSeries = transformVolatilityToTimeSeries



// All the other dashboard components to check their structure:

// frontend/src/components/zscore/dashboards/ZScoreOverview.vue
// frontend/src/components/zscore/dashboards/CrossAssetMatrix.vue
// frontend/src/components/zscore/dashboards/VolatilityLiquidity.vue
// frontend/src/components/zscore/dashboards/InterAssetZScore.vue
// frontend/src/components/zscore/dashboards/LeadersLaggards.vue
// frontend/src/components/zscore/dashboards/MarketMicrostructure.vue
// frontend/src/components/zscore/dashboards/RelativeStrength.vue
// frontend/src/components/zscore/dashboards/MarketRegime.vue
// Their corresponding CSS files to check for any width overrides:

// frontend/src/styles/components/zscore/overview.css
// frontend/src/styles/components/zscore/cross-asset.css
// frontend/src/styles/components/zscore/liquidity.css
// frontend/src/styles/components/zscore/inter-asset.css
// frontend/src/styles/components/zscore/leaders.css
// frontend/src/styles/components/zscore/microstructure.css
// frontend/src/styles/components/zscore/relative-strength.css
// frontend/src/styles/components/zscore/market-regime.css