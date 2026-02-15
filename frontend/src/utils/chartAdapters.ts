
// src/utils/chartAdapters.ts
import type { KlineDTO, RvPoint } from '@/utils/types'

export function toFinancialSeries(klines: KlineDTO[]) {
  return klines.map(k => ({ x: new Date(k.ts), o: k.open, h: k.high, l: k.low, c: k.close }))
}
export function toReturnSeries(points: RvPoint[]) {
  return points.map(p => ({ x: new Date(p.ts), y: p.ret }))
}
export function toVolSeries(points: RvPoint[]) {
  return points.map(p => ({ x: new Date(p.ts), y: p.vol ?? null }))
}


