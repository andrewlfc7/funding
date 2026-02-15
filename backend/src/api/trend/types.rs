use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct UniverseQuery {
    pub exchange: String,
    #[serde(alias = "marketType")]
    pub market_type: String,
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(default)]
    pub days: Option<usize>,
    #[serde(default, alias = "volWindow")]
    pub vol_window: Option<usize>,
    #[serde(default, alias = "minDecile")]
    pub min_decile: Option<u8>,
    #[serde(default)]
    pub timeframe: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SignalsBundleQuery {
    #[serde(flatten)]
    pub uni: super::types::UniverseQuery, // same struct you already use

    #[serde(default)]
    pub scope: Option<String>, // "symbol" | "universe" (default: symbol if symbol present, else universe)

    #[serde(default)]
    pub symbol: Option<String>, // for symbol scope + rolling

    #[serde(default)]
    pub signals: Option<String>, // comma list: "trend,momentum,ewmac,breakout,composite"

    #[serde(default)]
    pub roll_win: Option<usize>, // rolling window (TS corr), default 60
}

#[derive(Deserialize, Debug)]
pub struct SignalsCorrQuery {
    #[serde(flatten)]
    pub uni: UniverseQuery,

    #[serde(default)]
    pub symbol: Option<String>, // accepted too if caller passes here

    #[serde(default)]
    pub scope: Option<String>, // "symbol" | "universe"

    #[serde(default)]
    pub signals: Option<String>, // "trend,momentum,ewmac,breakout,composite"
}

#[derive(Debug, Deserialize, Clone)]
pub struct AssetQuery {
    pub exchange: String,
    pub market_type: String, // "spot" | "perps"
    #[serde(alias = "ticker", alias = "base", alias = "market_symbol")]
    pub symbol: String, // base (e.g. "BTC")
    pub days: Option<i32>,   // default 90
}

#[derive(Debug, Deserialize, Clone)]
pub struct VolParams {
    #[serde(flatten)]
    pub asset: AssetQuery,
    pub model: Option<String>,     // "stdev" (default) | "ema-blend"
    pub vol_window: Option<usize>, // stdev window (default 30)
    pub short_span: Option<usize>, // ema-blend S (default 30)
    pub long_span: Option<usize>,  // ema-blend L (default 120)
    pub blend: Option<f64>,        // [0,1], default 0.5
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExpectedParams {
    // compute signals on the fly (universe or single-asset)
    pub exchange: String,
    pub market_type: String,
    pub days: Option<i32>,
    pub symbol: Option<String>, // if present → single series, otherwise whole universe

    // combine weights; defaults: 0.30/0.30/0.30/0.10
    pub w_trend: Option<f64>,
    pub w_mom: Option<f64>,
    pub w_ew: Option<f64>,
    pub w_bo: Option<f64>,

    // expected return mapping from combined signal
    pub er_map: Option<String>, // "linear" (default) | "logistic"
    pub er_slope: Option<f64>,  // default 1.0
    pub er_bias: Option<f64>,   // default 0.0

    // target exposure mapping (sigmoid)
    pub sig_center: Option<f64>, // default 0.0
    pub sig_slope: Option<f64>,  // default 4.0
    pub sig_cap: Option<f64>,    // default 0.85 (sym cap)
    pub sig_bias: Option<f64>,   // default 0.0 (post-shift)
}

#[derive(Debug, Deserialize, Clone)]
pub struct RegrParams {
    pub exchange: String,
    pub market_type: String,
    pub days: Option<i32>,               // fit window
    pub horizon: Option<i32>,            // forward horizon in days (default 1)
    pub symbol: Option<String>,          // if present → TS regression; else XSec
    pub vol_universe_decile: Option<u8>, // default 3 (only for XSec)
    // toggles
    pub use_trend: Option<bool>,
    pub use_mom: Option<bool>,
    pub use_ew: Option<bool>,
    pub use_bo: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TrendPoint {
    pub ts: i64,        // epoch ms
    pub symbol: String, // base
    pub trend: f64,     // per-date z-scores
    pub momentum: f64,
    pub ewmac: f64,
    pub breakout: f64,
    pub composite: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SimplePoint {
    pub ts: i64,
    pub value: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExpectedPoint {
    pub ts: i64,
    pub symbol: String,
    pub combined_signal: f64,
    pub expected_return: f64,
    pub target_exposure: f64,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct Betas {
    pub trend: f64,
    pub momentum: f64,
    pub ewmac: f64,
    pub breakout: f64,
}

#[derive(Debug, Serialize, Clone)]
pub struct RegressionReport {
    pub scope: String, // "xsec" | "ts"
    pub exchange: String,
    pub market_type: String,
    pub horizon_days: i32,
    pub start_ts: i64,
    pub end_ts: i64,
    pub n: usize, // observations used

    pub intercept: f64,
    pub betas: Betas,
    pub r2: f64,
    pub adj_r2: f64,
}
