use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{Error as DeError, SeqAccess, Visitor};
use sqlx::PgPool;
use std::fmt;
use std::collections::{BTreeMap, BTreeSet};

use super::{
    fetch_multi_hourly_ohlcv, log_returns, parse_period_days, resample_from_hourly,
    top_markets_by_usd_volume_live, Tf,
};

// ---------- Configuration & Helpers ----------

fn default_market_type() -> String { "spot".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }

// Treat common and variant tickers as stables (extend as needed)
fn is_stable_symbol(sym: &str) -> bool {
    let s = sym.trim().to_ascii_uppercase();
    // common stables & variants
    const STABLES: &[&str] = &[
        "USD","USDT","USDC","FDUSD","FUSD","TUSD","BUSD","DAI","PYUSD","USDE","USDD","USDP",
        "GUSD","USDJ","USDX","FRAX","LUSD","SUSD","MIM","DOLA","EURS","EURC","EURT","CRVUSD",
        "USDL",
    ];
    if STABLES.contains(&s.as_str()) { return true; }
    // generic catch-alls (covers many chain-specific stables)
    s.ends_with("USD") || s.ends_with("USDT") || s.ends_with("USDC")
}

// Accept string, CSV string, or sequence (?compareCoins=A&compareCoins=B)
fn string_or_seq<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where D: Deserializer<'de> {
    struct StrOrSeq;
    impl<'de> Visitor<'de> for StrOrSeq {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "string, CSV string, or sequence of strings")
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: DeError {
            Ok(v.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).map(String::from).collect())
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
            let mut out = Vec::new();
            while let Some(elem) = seq.next_element::<String>()? {
                out.extend(elem.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).map(String::from));
            }
            Ok(out)
        }
    }
    de.deserialize_any(StrOrSeq)
}

// ---------- Request DTO ----------

#[derive(Debug, Deserialize)]
pub struct CrossAssetRequest {
    #[serde(default)]
    pub indexCoin: Option<String>,
    #[serde(default, deserialize_with = "string_or_seq")]
    pub compareCoins: Vec<String>,
    pub period: String,
    pub window: usize,
    pub exchange: String,
    #[serde(default = "default_market_type")]
    pub marketType: String,
    #[serde(default)]
    pub topN: Option<i64>,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
}

// ---------- Unified Response DTOs (without histograms) ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossAssetAnalyticsResponse {
    pub correlation_matrix: CorrMatrix,
    pub beta_matrix: BetaMatrix,
    pub index: String,
    pub rolling_corr: Vec<Vec<f64>>,
    pub rolling_beta: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct CorrMatrix {
    pub coins: Vec<String>,
    pub matrix: Vec<Vec<f64>>,
    pub timestamp: i64,
}

#[derive(Debug, Serialize)]
pub struct BetaMatrix {
    pub coins: Vec<String>,
    pub betas: Vec<f64>,
}

// ---------- Calculation Helpers ----------

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len();
    if n == 0 || n != y.len() { return 0.0; }
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut m = 0usize;
    for i in 0..n {
        let (xi, yi) = (x[i], y[i]);
        if xi.is_finite() && yi.is_finite() {
            sx += xi; sy += yi; sxx += xi*xi; syy += yi*yi; sxy += xi*yi; m += 1;
        }
    }
    if m <= 1 { return 0.0; }
    let m_f = m as f64;
    let cov = sxy - sx * sy / m_f;
    let vx  = sxx - sx * sx / m_f;
    let vy  = syy - sy * sy / m_f;
    if vx <= 0.0 || vy <= 0.0 { return 0.0; }
    finite(cov / (vx.sqrt() * vy.sqrt()))
}

fn beta_vs(x: &[f64], idx: &[f64]) -> f64 {
    let n = x.len();
    if n == 0 || n != idx.len() { return 0.0; }
    let (mut sx, mut si, mut sxi, mut sii) = (0.0, 0.0, 0.0, 0.0);
    let mut m = 0usize;
    for i in 0..n {
        let (xi, ii) = (x[i], idx[i]);
        if xi.is_finite() && ii.is_finite() {
            sx += xi; si += ii; sxi += xi * ii; sii += ii * ii; m += 1;
        }
    }
    if m <= 1 { return 0.0; }
    let m_f = m as f64;
    let cov = sxi - sx * si / m_f;
    let var = sii - si * si / m_f;
    if var <= 0.0 { return 0.0; }
    finite(cov / var)
}

// ---------- Handler (stablecoin-filtered) ----------

pub async fn get_cross_asset_analytics(
    State(pool): State<PgPool>,
    Query(q): Query<CrossAssetRequest>,
) -> Json<CrossAssetAnalyticsResponse> {
    // 1) Setup
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    let days = parse_period_days(&q.period);
    let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();

    // guard index; fall back if it's a stable
    let mut index_sym = q.indexCoin.clone().unwrap_or_else(|| "BTC".to_string()).to_uppercase();
    if is_stable_symbol(&index_sym) {
        index_sym = "BTC".to_string();
    }

    // build universe (explicit + topN), uppercase, then drop stables
    let mut sym_set: BTreeSet<String> = q.compareCoins.iter().map(|s| s.to_uppercase()).collect();

    if let Some(n) = q.topN {
        if let Ok(top) = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.marketType, days as i32, n).await {
            sym_set.extend(top.into_iter().map(|(sym, _, _)| sym.to_uppercase()));
        }
    }

    // always include index
    sym_set.insert(index_sym.clone());
    // remove stables (compareCoins or topN may include them)
    sym_set = sym_set.into_iter().filter(|s| !is_stable_symbol(s)).collect();

    // need at least 2 symbols after filtering
    if sym_set.len() < 2 {
        return Json(CrossAssetAnalyticsResponse::default());
    }

    let all_symbols: Vec<String> = sym_set.into_iter().collect();

    // 2) Resolve market ids + hourly data
    let mut mids = Vec::new();
    let mut sym_mid_map = Vec::new();
    for sym in &all_symbols {
        if let Ok(mid) = super::resolve_market_id_with_data(&pool, &q.exchange, sym, &q.marketType, since_unix).await {
            mids.push(mid);
            sym_mid_map.push((sym.clone(), mid));
        }
    }

    let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix).await.unwrap_or_default();

    // 3) Resample + compute log returns
    let mut resampled_data = BTreeMap::new(); // mid -> (sym, ts, lr)
    for (sym, mid) in &sym_mid_map {
        if let Some(hourly) = by_mid.get(mid) {
            let series = resample_from_hourly(hourly, tf.period_secs());
            if series.len() > q.window {
                let closes: Vec<f64> = series.iter().map(|r| r.close).collect();
                let lr = log_returns(&closes);
                if !lr.is_empty() {
                    let ts: Vec<i64> = series.iter().map(|r| r.ts).collect();
                    resampled_data.insert(mid, (sym.clone(), ts, lr));
                }
            }
        }
    }

    if resampled_data.len() < 2 { return Json(CrossAssetAnalyticsResponse::default()); }

    // 4) Align by common timestamps
    let mut common_timestamps = resampled_data.values().next().unwrap().1.clone();
    for data in resampled_data.values().skip(1) {
        let mut intersection = Vec::new();
        let (mut i, mut j) = (0, 0);
        while i < common_timestamps.len() && j < data.1.len() {
            if common_timestamps[i] == data.1[j] {
                intersection.push(common_timestamps[i]); i += 1; j += 1;
            } else if common_timestamps[i] < data.1[j] { i += 1; } else { j += 1; }
        }
        common_timestamps = intersection;
    }

    let mut aligned_data = BTreeMap::new(); // mid -> (sym, lr_aligned)
    for (mid, (sym, ts, lr)) in resampled_data {
        let mut new_lr = Vec::with_capacity(common_timestamps.len());
        let (mut i, mut j) = (0, 0);
        while i < ts.len() && j < common_timestamps.len() {
            if ts[i] == common_timestamps[j] {
                new_lr.push(lr[i]); j += 1;
            }
            i += 1;
        }
        if new_lr.len() >= q.window {
            aligned_data.insert(mid, (sym, new_lr));
        }
    }

    if aligned_data.len() < 2 { return Json(CrossAssetAnalyticsResponse::default()); }

    let all_symbols_final: Vec<String> = aligned_data.values().map(|(s, _)| s.clone()).collect();
    let all_returns_final: Vec<&Vec<f64>> = aligned_data.values().map(|(_, lr)| lr).collect();

    // ensure index exists post-filter; else fallback to first
    let index_pos = all_symbols_final.iter().position(|s| *s == index_sym).unwrap_or(0);
    let index_returns = &all_returns_final[index_pos];
    if index_returns.len() < q.window { return Json(CrossAssetAnalyticsResponse::default()); }

    let steps = index_returns.len().saturating_sub(q.window) + 1;

    // 5) Rolling stats
    let mut rolling_corr = vec![vec![0.0; steps]; all_symbols_final.len()];
    let mut rolling_beta = vec![vec![0.0; steps]; all_symbols_final.len()];

    for i in 0..steps {
        let w0 = i;
        let w1 = w0 + q.window;
        let idx_slice = &index_returns[w0..w1];
        for (j, coin_returns) in all_returns_final.iter().enumerate() {
            let coin_slice = &coin_returns[w0..w1];
            rolling_corr[j][i] = pearson(coin_slice, idx_slice);
            rolling_beta[j][i] = beta_vs(coin_slice, idx_slice);
        }
    }

    // 6) Matrices from last window
    let last_window_start = index_returns.len() - q.window;
    let last_ts = *common_timestamps.last().unwrap_or(&0);

    let mut corr_matrix_values = vec![vec![0.0; all_symbols_final.len()]; all_symbols_final.len()];
    for i in 0..all_symbols_final.len() {
        for j in i..all_symbols_final.len() {
            let c = pearson(&all_returns_final[i][last_window_start..],
                            &all_returns_final[j][last_window_start..]);
            corr_matrix_values[i][j] = c;
            corr_matrix_values[j][i] = c;
        }
    }

    let final_betas: Vec<f64> = all_returns_final.iter()
        .map(|s| beta_vs(&s[last_window_start..], &index_returns[last_window_start..]))
        .collect();

    // 7) Response
    Json(CrossAssetAnalyticsResponse {
        correlation_matrix: CorrMatrix { coins: all_symbols_final.clone(), matrix: corr_matrix_values, timestamp: last_ts },
        beta_matrix: BetaMatrix { coins: all_symbols_final.clone(), betas: final_betas },
        index: index_sym,
        rolling_corr,
        rolling_beta,
    })
}

// Default implementation for empty/error responses
impl Default for CrossAssetAnalyticsResponse {
    fn default() -> Self {
        Self {
            correlation_matrix: CorrMatrix { coins: vec![], matrix: vec![], timestamp: 0 },
            beta_matrix: BetaMatrix { coins: vec![], betas: vec![] },
            index: "BTC".to_string(),
            rolling_corr: vec![],
            rolling_beta: vec![],
        }
    }
}

