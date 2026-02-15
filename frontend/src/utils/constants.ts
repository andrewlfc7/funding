const env = import.meta.env

// ---- Base ----
export const API_BASE_URL = (env.VITE_API_URL as string) || ''

// ---- Main App endpoints ----
export const FUNDING_ENDPOINT = (env.VITE_FUNDING_ENDPOINT as string) || '/api/funding/matrix'
export const OPTIONS_ENDPOINT = (env.VITE_OPTIONS_ENDPOINT as string) || '/api/meta-data/options'

// ---- Trend API Endpoints ----
export const TREND_SIGNALS_XSEC_ENDPOINT = (env.VITE_TREND_SIGNALS_XSEC_ENDPOINT as string) || '/api/trend/signals/xsec'
export const TREND_EXPECTED_RETURNS_ENDPOINT = (env.VITE_TREND_EXPECTED_RETURNS_ENDPOINT as string) || '/api/trend/expected'
export const TREND_REGRESSIONS_XSEC_ENDPOINT = (env.VITE_TREND_REGRESSIONS_XSEC_ENDPOINT as string) || '/api/trend/regressions/xsec'
export const TREND_REGRESSIONS_TS_ENDPOINT = (env.VITE_TREND_REGRESSIONS_TS_ENDPOINT as string) || '/api/trend/regressions/ts'
export const TREND_SERIES_PRICE_ENDPOINT = (env.VITE_TREND_SERIES_PRICE_ENDPOINT as string) || '/api/trend/series/price'
export const TREND_SERIES_RETURNS_ENDPOINT = (env.VITE_TREND_SERIES_RETURNS_ENDPOINT as string) || '/api/trend/series/returns'
export const TREND_SERIES_VOL_ENDPOINT = (env.VITE_TREND_SERIES_VOL_ENDPOINT as string) || '/api/trend/series/vol'
export const TREND_SERIES_VOLUME_ENDPOINT = (env.VITE_TREND_SERIES_VOLUME_ENDPOINT as string) || '/api/trend/series/volume'
export const TREND_PORTFOLIO_ENDPOINT = (env.VITE_TREND_PORTFOLIO_ENDPOINT as string) || '/api/trend/portfolio'
export const TREND_PORTFOLIO_CONSTRAINTS_ENDPOINT = (env.VITE_TREND_PORTFOLIO_CONSTRAINTS_ENDPOINT as string) || '/api/trend/portfolio/constraints'


// ---- Refresh & misc ----
export const REFRESH_INTERVAL = Number(env.VITE_REFRESH_INTERVAL ?? 300)
export const SELECTED_QUOTE = (env.VITE_SELECTED_QUOTE as string) || 'USDT'

// ---- Statistics (prefer VITE_STATS_*; keep compat fallbacks where needed) ----
export const ZSCORE_OVERVIEW_ENDPOINT =
  (env.VITE_STATS_ZSCORE_OVERVIEW as string) ||
  (env.VITE_ZSCORE_OVERVIEW_ENDPOINT as string) ||
  '/api/statistics/zscore/overview'

export const ZSCORE_VOLATILITY_ENDPOINT =
  (env.VITE_STATS_VOL_ANALYSIS as string) ||
  (env.VITE_ZSCORE_VOLATILITY_ENDPOINT as string) ||
  '/api/statistics/volatility/analysis'

export const ZSCORE_CROSS_ASSET_ENDPOINT =
  (env.VITE_STATS_CROSS_ASSET as string) ||
  '/api/statistics/cross-asset/analytics'

export const ZSCORE_INTER_ASSET_ENDPOINT =
  (env.VITE_STATS_INTER_ASSET_Z as string) ||
  (env.VITE_ZSCORE_INTER_ASSET_ENDPOINT as string) ||
  '/api/statistics/inter-asset/zscore'

export const ZSCORE_VOL_LIQ_ENDPOINT =
  (env.VITE_STATS_VOL_LIQ as string) ||
  (env.VITE_ZSCORE_VOL_LIQ_ENDPOINT as string) ||
  '/api/statistics/volatility/liquidity'

export const LEADERS_LAGGARDS_ENDPOINT =
  (env.VITE_STATS_LEADERS_LAGGARDS as string) ||
  (env.VITE_ZSCORE_LEADERS_LAGGARDS_ENDPOINT as string) ||
  '/api/statistics/cross-section/leaders-laggards'

export const ZSCORE_MICROSTRUCTURE_ENDPOINT =
  (env.VITE_STATS_MICROSTRUCTURE as string) ||
  (env.VITE_ZSCORE_MICROSTRUCTURE_ENDPOINT as string) ||
  '/api/statistics/microstructure/flow'

// ---- New statistics endpoints ----
export const ZSCORE_RELATIVE_STRENGTH_ENDPOINT =
  (env.VITE_STATS_RELATIVE_STRENGTH as string) ||
  '/api/statistics/relative-strength/overview'

export const ZSCORE_REGIME_MOMENTUM_ENDPOINT =
  (env.VITE_STATS_REGIME_MOMENTUM as string) ||
  '/api/statistics/momentum/regime'

export const ZSCORE_MARKET_SEASONALITY_ENDPOINT =
  (env.VITE_STATS_MARKET_SEASONALITY as string) ||
  '/api/zscore/market-seasonality'

export const ZSCORE_VOLATILITY_DYNAMICS_ENDPOINT =
  (env.VITE_STATS_VOLATILITY_DYNAMICS as string) ||
  '/api/zscore/volatility-dynamics'

export const ZSCORE_TRADES_ANALYSIS_ENDPOINT =
  (env.VITE_STATS_TRADES_ANALYSIS as string) ||
  '/api/zscore/trades-analysis'

// ---- Aliases for older imports ----
export const RS_OVERVIEW_ENDPOINT = ZSCORE_RELATIVE_STRENGTH_ENDPOINT
export const MARKET_REGIME_ENDPOINT = ZSCORE_REGIME_MOMENTUM_ENDPOINT

// ---- helpers ----
export function joinUrl(base: string, path: string) {
  if (!base) return path
  if (!path) return base
  const b = base.endsWith('/') ? base.slice(0, -1) : base
  const p = path.startsWith('/') ? path : `/${path}`
  return `${b}${p}`
}

// ---- sanity checks ----
if (!API_BASE_URL) console.error('[ENV] VITE_API_URL is missing')
if (!FUNDING_ENDPOINT) console.error('[ENV] funding endpoint missing')
if (!Number.isFinite(REFRESH_INTERVAL)) console.error('[ENV] VITE_REFRESH_INTERVAL must be numeric')
if (!ZSCORE_OVERVIEW_ENDPOINT) console.error('[ENV] zscore overview endpoint missing')
if (!ZSCORE_VOLATILITY_ENDPOINT) console.error('[ENV] zscore volatility endpoint missing')

// ---- domain constants ----
export const FUNDING_PERIODS_PER_DAY = 3
export const DAYS_PER_YEAR = 365

export const MIN_SPREAD_THRESHOLD_BPS = 4
export const SPREAD_THRESHOLDS = {
  EXCELLENT: 100,
  GOOD: 30,
  DECENT: 10,
  HIGH: 80,
  MEDIUM: 60,
}

export const PERIOD_TO_DAYS: Record<string, number> = {
  '30d': 30,
  '90d': 90,
  '180d': 180,
  '1y': 365,
}
