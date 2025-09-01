use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{Error as DeError, SeqAccess, Visitor};
use sqlx::PgPool;
use std::fmt;

use super::{
    fetch_multi_hourly_ohlcv, log_returns, parse_period_days, resample_from_hourly,
    top_markets_by_usd_volume_live, Tf,
};

fn default_market_type() -> String { "spot".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }

// Accept string, CSV string, or sequence (?compareCoins=A&compareCoins=B)
fn string_or_seq<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where D: Deserializer<'de> {
    struct StrOrSeq;
    impl<'de> Visitor<'de> for StrOrSeq {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "string, CSV string, or sequence of strings")
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where E: DeError {
            let parts: Vec<String> = v.split(',')
                .map(|s| s.trim()).filter(|s| !s.is_empty())
                .map(|s| s.to_string()).collect();
            Ok(if parts.is_empty() { vec![v.to_string()] } else { parts })
        }
        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where E: DeError { self.visit_str(&v) }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de> {
            let mut out = Vec::new();
            while let Some(elem) = seq.next_element::<String>()? {
                if elem.contains(',') {
                    out.extend(elem.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).map(|s| s.to_string()));
                } else {
                    out.push(elem);
                }
            }
            Ok(out)
        }
    }
    de.deserialize_any(StrOrSeq)
}

// ---------- Request / Response ----------

#[derive(Debug, Deserialize)]
pub struct CrossAssetMatrixRequest {
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

#[derive(Debug, Serialize)]
pub struct CrossAssetMatrixResponse {
    pub correlationMatrix: CorrMatrix,
    pub betaMatrix: BetaMatrix,
    pub correlationHistogram: StatsHist,
    pub betaHistogram: StatsHist,
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
    pub betas: Vec<f64>, // beta vs index
}

#[derive(Debug, Serialize)]
pub struct StatsHist {
    pub buckets: Vec<f64>,
    pub counts: Vec<usize>,
    pub mean: f64,
    pub std: f64,
}

// ---------- helpers ----------

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n == 0 { return 0.0; }
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
    let cov = sxy - sx*sy/m_f;
    let vx  = sxx - sx*sx/m_f;
    let vy  = syy - sy*sy/m_f;
    if vx <= 0.0 || vy <= 0.0 { return 0.0; }
    finite(cov / (vx.sqrt()*vy.sqrt()))
}

fn beta_vs(x: &[f64], idx: &[f64]) -> f64 {
    let n = x.len().min(idx.len());
    if n == 0 { return 0.0; }
    let (mut sx, mut si, mut sxx, mut sii, mut sxi) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut m = 0usize;
    for i in 0..n {
        let (xi, ii) = (x[i], idx[i]);
        if xi.is_finite() && ii.is_finite() {
            sx += xi; si += ii; sxx += xi*xi; sii += ii*ii; sxi += xi*ii; m += 1;
        }
    }
    if m <= 1 { return 0.0; }
    let m_f = m as f64;
    let cov = sxi - sx*si/m_f;
    let var = sii - si*si/m_f;
    if var <= 0.0 { return 0.0; }
    finite(cov / var)
}

fn histogram(xs: &[f64], start: f64, stop: f64, step: f64) -> (Vec<f64>, Vec<usize>, f64, f64) {
    let mut buckets = Vec::new();
    let mut v = start;
    while v <= stop + 1e-12 { buckets.push(v); v += step; }
    let mut counts = vec![0usize; buckets.len()];
    let mut sum = 0.0; let mut sumsq = 0.0; let mut m = 0usize;
    let half = step/2.0;
    for &x in xs {
        if !x.is_finite() { continue; }
        sum += x; sumsq += x*x; m += 1;
        for (i, &b) in buckets.iter().enumerate() {
            if x >= b - half && x < b + half { counts[i] += 1; break; }
        }
    }
    let mean = if m>0 { sum / (m as f64) } else { 0.0 };
    let var  = if m>0 { sumsq/(m as f64) - mean*mean } else { 0.0 };
    (buckets, counts, mean, var.max(0.0).sqrt())
}

// ---------- handler ----------

pub async fn get_cross_asset_matrix(
    State(pool): State<PgPool>,
    Query(q): Query<CrossAssetMatrixRequest>,
) -> Json<CrossAssetMatrixResponse> {
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    let days = parse_period_days(&q.period);
    let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();


    // --- inside handler, right after you parse q/timeframe/etc. ---

    // 0) Decide the index (default BTC) *before* building the universe
    let idx_name = q.indexCoin
        .clone()
        .unwrap_or_else(|| "BTC".to_string())
        .to_uppercase();

    // 1) Build universe
    let mut sym_set: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // (a) explicit list if provided
    for s in &q.compareCoins {
        sym_set.insert(s.to_uppercase());
    }

    // (b) topN (volume-ranked) if requested
    if let Some(n) = q.topN {
        if let Ok(top) = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.marketType, days as i32, n).await {
            // NOTE: ensure `top` yields BASE symbols (e.g., ETH), not full market codes.
            for (sym, _, _) in top {
                sym_set.insert(sym.to_uppercase());
            }
        }
    }

    // (c) ensure index coin is present even if not in compareCoins/topN
    sym_set.insert(idx_name.clone());

    // (d) final fallback if still empty (e.g., no topN and no compareCoins were valid)
    if sym_set.is_empty() {
        if let Ok(top) = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.marketType, days as i32, 30).await {
            for (sym, _, _) in top {
                sym_set.insert(sym.to_uppercase());
            }
            // include index again to be safe
            sym_set.insert(idx_name.clone());
        }
    }

    let symbols: Vec<String> = sym_set.into_iter().collect();

    // 2) Resolve market IDs (be lenient on since_unix if it’s too strict)
    let mut mids = Vec::new();
    let mut sym_to_mid = Vec::new();
    for sym in &symbols {
        // If resolve filters out due to since_unix gaps, try a looser lookup:
        if let Ok(mid) = super::resolve_market_id_with_data(&pool, &q.exchange, sym, &q.marketType, since_unix).await {
            mids.push(mid);
            sym_to_mid.push((sym.clone(), mid));
        } else if let Ok(mid_any) = super::resolve_market_id_with_data(&pool, &q.exchange, sym, &q.marketType, 0).await {
            // fallback: allow any data horizon
            mids.push(mid_any);
            sym_to_mid.push((sym.clone(), mid_any));
        }
    }

    let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix).await.unwrap_or_default();

    // 3) Convert to returns; relax the length requirement
    let mut coin_returns: Vec<(String, Vec<f64>, i64)> = Vec::new();
    for (sym, mid) in sym_to_mid {
        if let Some(hourly) = by_mid.get(&mid) {
            let series = resample_from_hourly(hourly, tf.period_secs());
            // require at least enough points to compute some returns
            if series.len() < 3 { continue; }

            let close: Vec<f64> = series.iter().map(|r| r.close).collect();
            let lr = log_returns(&close);
            if lr.len() < 2 { continue; }

            let last_ts = series.last().map(|r| r.ts).unwrap_or(since_unix);
            // prefer last `window`, but accept shorter if still meaningful
            let want = q.window;
            let have = lr.len();
            let start = have.saturating_sub(want);
            let slice = &lr[start..have];

            // skip only if slice is truly too short to correlate/beta (need >= 2)
            if slice.len() < 2 { continue; }

            coin_returns.push((sym, slice.to_vec(), last_ts));
        }
    }

    // 4) hard fail with a message if empty (better than silent empty JSON)
    if coin_returns.is_empty() {
        // You can define a proper error type; for brevity returning empty JSON with a clear ts:
        let ts = since_unix;
        return Json(CrossAssetMatrixResponse {
            correlationMatrix: CorrMatrix { coins: vec![], matrix: vec![], timestamp: ts },
            betaMatrix: BetaMatrix { coins: vec![], betas: vec![] },
            correlationHistogram: StatsHist { buckets: vec![], counts: vec![], mean: 0.0, std: 0.0 },
            betaHistogram: StatsHist { buckets: vec![], counts: vec![], mean: 0.0, std: 0.0 },
        });
    }

    // 5) Reorder so index coin is first
    let mut coins: Vec<String> = coin_returns.iter().map(|(s, _, _)| s.clone()).collect();
    let ts_last = coin_returns.iter().map(|(_,_,ts)| *ts).max().unwrap_or(since_unix);
    let idx_pos = coins.iter().position(|s| s == &idx_name).unwrap_or(0);
    if idx_pos != 0 { coins.swap(0, idx_pos); coin_returns.swap(0, idx_pos); }

    // (rest of your corr/beta/hist logic unchanged)




    // Corr matrix
    let m = coin_returns.len();
    let mut mat = vec![vec![0.0; m]; m];
    for i in 0..m {
        for j in i..m {
            let c = pearson(&coin_returns[i].1, &coin_returns[j].1);
            mat[i][j] = c; mat[j][i] = c;
        }
    }

    // Betas vs index
    let idx_ret = &coin_returns[0].1;
    let mut betas = vec![0.0; m];
    for i in 0..m {
        betas[i] = beta_vs(&coin_returns[i].1, idx_ret);
    }

    // Hists
    let mut corr_flat: Vec<f64> = Vec::new();
    for i in 0..m { for j in (i+1)..m { corr_flat.push(mat[i][j]); } }
    let (corr_b, corr_c, corr_mean, corr_std) = histogram(&corr_flat, -1.0, 1.0, 0.1);
    let (beta_b, beta_c, beta_mean, beta_std) = histogram(&betas, -3.0, 3.0, 0.25);

    Json(CrossAssetMatrixResponse {
        correlationMatrix: CorrMatrix { coins, matrix: mat, timestamp: ts_last },
        betaMatrix: BetaMatrix {
            coins: coin_returns.iter().map(|(s,_,_)| s.clone()).collect(),
            betas
        },
        correlationHistogram: StatsHist { buckets: corr_b, counts: corr_c, mean: finite(corr_mean), std: finite(corr_std) },
        betaHistogram: StatsHist { buckets: beta_b, counts: beta_c, mean: finite(beta_mean), std: finite(beta_std) },
    })
}





