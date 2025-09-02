import axios from 'axios'
import { API_BASE_URL, RS_OVERVIEW_ENDPOINT, joinUrl } from '@/utils/constants'
import { handleApiError } from '@/api/error'
import type {
  MarketType,
  RelativeStrengthResponseRaw,
  RelativeStrengthResponse,
  RSRankRow, RSSeriesRow, RSPairDivergence
} from './types'

// helpers
const firstNonNull = <T>(...vals: Array<T | null | undefined>) =>
  vals.find(v => v !== null && v !== undefined)

function normalizeRankings(input: RelativeStrengthResponseRaw['rsRankings']): RSRankRow[] {
  if (!input) return []
  const arr = Array.isArray(input) ? input : Object.values(input)
  const out = arr.map((r, idx) => ({
    rank: (r.rank ?? (idx + 1)) as number,
    symbol: (firstNonNull(r.symbol, r.pair, r.name) ?? '') as string,
    z: (firstNonNull(r.z, r.zscore, r.score, r.rsZScore, r.relativeStrengthZ) ?? null) as number | null
  }))
  out.sort((a,b) => (a.rank ?? 1e9) - (b.rank ?? 1e9))
  return out
}
function normalizeSeries(input: RelativeStrengthResponseRaw['rsSeries']): RSSeriesRow[] {
  if (!input) return []
  const raw = Array.isArray(input) ? input : Object.values(input)
  return raw.map(r => ({
    symbol: (firstNonNull(r.symbol, r.pair, r.name) ?? '') as string,
    series: (r.series ?? r.timeSeries ?? []),
  }))
}
function normalizePairDiv(input: RelativeStrengthResponseRaw['pairDivergence']): RSPairDivergence[] {
  if (!input) return []
  return input.map(pd => ({
    pair: pd.pair,
    timeSeries: (pd.timeSeries ?? pd.series ?? []),
  }))
}

function normalize(raw: RelativeStrengthResponseRaw): RelativeStrengthResponse {
  return {
    base: raw.base ?? raw.baseCoin ?? '',
    momentumFactorLoadings: raw.momentumFactorLoadings ?? [],
    pairDivergence: normalizePairDiv(raw.pairDivergence),
    persistence: raw.persistence ?? [],
    rsRankings: normalizeRankings(raw.rsRankings),
    rsSeries: normalizeSeries(raw.rsSeries),
  }
}

export async function fetchRelativeStrength(params: {
  exchange: string
  base: string
  period: string
  marketType?: MarketType
  topN?: number
}): Promise<RelativeStrengthResponse> {
  try {
    const url = joinUrl(API_BASE_URL, RS_OVERVIEW_ENDPOINT)
    const { data } = await axios.get<RelativeStrengthResponseRaw>(url, {
      params: { marketType: 'spot', ...params },
    })
    return normalize(data)
  } catch (err) {
    throw handleApiError(err)
  }
}
