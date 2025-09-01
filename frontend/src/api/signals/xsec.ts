// src/api/signals/xsec.ts
import { API_BASE_URL, SIGNALS_ENDPOINT, joinUrl } from '@/utils/constants'

export type MarketType = 'spot' | 'perps'

export interface XSecParams {
  exchange: string
  market_type: MarketType
  days?: number
  vol_window?: number
  min_decile?: number
  /** NEW: filter to one or more symbols (comma-separated), e.g. "BTCUSDT,ETHUSDT" */
  symbol?: string
}

export interface XSecRow {
  ts: number
  symbol: string
  trend: number
  momentum: number
  ewmac: number
  breakout: number
  composite: number
}

export async function fetchXSecSignals(params: XSecParams): Promise<XSecRow[]> {
  const url = joinUrl(API_BASE_URL, SIGNALS_ENDPOINT)
  // normalize symbol(s) to uppercase (DB usually stores upper)
  const p = { ...params, market_symbol: params.symbol?.toUpperCase() }
  const qs = new URLSearchParams(p as any).toString()
  const res = await fetch(`${url}?${qs}`, { credentials: 'same-origin' })
  if (!res.ok) throw new Error(`xsec fetch failed: ${res.status} ${res.statusText}`)
  const data: { rows: XSecRow[] } = await res.json()
  return data.rows
}



export function buildSeries(rows: XSecRow[], symbol: string, ids: string[]) {
  const r = rows.filter(d => d.symbol === symbol).sort((a, b) => a.ts - b.ts)
  const labels = r.map(d => new Date(d.ts))
  const dataById: Record<string, number[]> = {}
  ids.forEach(id => { dataById[id] = r.map(d => (d as any)[id] as number) })
  return { labels, dataById }
}

export function latestCrossSection(rows: XSecRow[]) {
  if (!rows.length) return { ts: null as number | null, data: [] as XSecRow[] }
  const lastTs = rows.reduce((m, r) => (r.ts > m ? r.ts : m), rows[0].ts)
  return { ts: lastTs, data: rows.filter(r => r.ts === lastTs) }
}


