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


