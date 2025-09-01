// src/utils/constants.ts
const env = import.meta.env

export const API_BASE_URL =
  (env.VITE_API_URL as string) || ''

export const FUNDING_ENDPOINT =
  (env.VITE_FUNDING_ENDPOINT as string) || '/api/funding/matrix'

export const KLINES_ENDPOINT =
  (env.VITE_KLINES_ENDPOINT as string) || '/api/klines/daily'

export const RETURNS_ENDPOINT =
  (env.VITE_RETURNS_ENDPOINT as string) || '/api/signals/returns'

export const VOLATILITY_ENDPOINT =
  (env.VITE_VOLATILITY_ENDPOINT as string) || '/api/signals/volatility'

export const REFRESH_INTERVAL = Number(env.VITE_REFRESH_INTERVAL ?? 30000)
export const SELECTED_QUOTE = (env.VITE_SELECTED_QUOTE as string) || 'USDT'
export const OPTIONS_ENDPOINT =
  (env.VITE_OPTIONS_ENDPOINT as string) || ''

export const SIGNALS_ENDPOINT =
  (env.VITE_SIGNALS_ENDPOINT as string) || '/api/signals/xsec'

// ---- STATISTICS endpoints ----
export const ZSCORE_OVERVIEW_ENDPOINT =
  (env.VITE_ZSCORE_OVERVIEW_ENDPOINT as string) || '/api/statistics/zscore/overview'

export const ZSCORE_VOLATILITY_ENDPOINT =
  (env.VITE_ZSCORE_VOLATILITY_ENDPOINT as string) || '/api/statistics/volatility/analysis'

export const ZSCORE_CROSS_ASSET_ENDPOINT =
  (env.VITE_ZSCORE_CROSS_ASSET_ENDPOINT as string) || '/api/statistics/cross-asset/matrix'

export const ZSCORE_INTER_ASSET_ENDPOINT =
  (env.VITE_ZSCORE_INTER_ASSET_ENDPOINT as string) || '/api/statistics/inter-asset/zscore'

export const ZSCORE_VOL_LIQ_ENDPOINT =
  (env.VITE_ZSCORE_VOL_LIQ_ENDPOINT as string) || '/api/statistics/volatility/liquidity'

export const ZSCORE_LEADERS_LAGGARDS_ENDPOINT =
  (env.VITE_ZSCORE_LEADERS_LAGGARDS_ENDPOINT as string) || '/api/statistics/cross-section/leaders-laggards'

export const ZSCORE_MICROSTRUCTURE_ENDPOINT =
  (env.VITE_ZSCORE_MICROSTRUCTURE_ENDPOINT as string) || '/api/statistics/microstructure/flow'

// ---- helpers & sanity checks ----
export function joinUrl(base: string, path: string) {
  if (!base) return path
  if (!path) return base
  const b = base.endsWith('/') ? base.slice(0, -1) : base
  const p = path.startsWith('/') ? path : `/${path}`
  return `${b}${p}`
}

if (!API_BASE_URL) console.error('[ENV] VITE_API_URL is missing')
if (!FUNDING_ENDPOINT) console.error('[ENV] funding endpoint missing')
if (!KLINES_ENDPOINT) console.error('[ENV] klines endpoint missing')
if (!RETURNS_ENDPOINT) console.error('[ENV] rv endpoint missing')
if (!Number.isFinite(REFRESH_INTERVAL)) console.error('[ENV] VITE_REFRESH_INTERVAL must be numeric')
if (!ZSCORE_OVERVIEW_ENDPOINT) console.error('[ENV] zscore overview endpoint missing')
if (!ZSCORE_VOLATILITY_ENDPOINT) console.error('[ENV] zscore volatility endpoint missing')

// domain constants
export const FUNDING_PERIODS_PER_DAY = 3 
export const DAYS_PER_YEAR = 365

export const MIN_SPREAD_THRESHOLD_BPS = 1.5
export const SPREAD_THRESHOLDS = {
  EXCELLENT: 30,
  GOOD: 8,
  DECENT: 4,
  HIGH: 10,
  MEDIUM: 5
}

export const PERIOD_TO_DAYS: Record<string, number> = {
  '30d': 30,
  '90d': 90,
  '180d': 180,
  '1y': 365,
}
