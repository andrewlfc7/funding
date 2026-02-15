// src/api/trend/risk.rs
use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tracing::{error, info};

use super::shared::{DailyBar, fetch_universe_daily};
use super::types::UniverseQuery;

// ⬇️ your log-returns helper lives in api/statistics/mod.rs
// adjust the path if your module structure differs
use crate::api::statistics::log_returns;

/* --------------------------- Query / DTOs --------------------------- */

#[derive(Debug, Deserialize)]
pub struct RiskMatrixQuery {
    #[serde(flatten)]
    pub uni: UniverseQuery, // uses: exchange, market_type, days, vol_window, min_decile, timeframe

    #[serde(default)]
    pub method: Option<String>, // "sample" | "ewma" (default: "sample")

    #[serde(default)]
    pub lambda: Option<f64>, // EWMA decay (default 0.94 when method=ewma)

    #[serde(default)]
    pub annualize: Option<bool>, // default false
}

#[derive(Debug, Serialize)]
pub struct RiskMatrixOut {
    pub method: String,
    pub annualized: bool,
    pub n_effective: usize,
    pub symbols: Vec<String>,                          // order used
    pub vols: BTreeMap<String, f64>,                   // per-asset stdev (ann if annualized=true)
    pub corr: BTreeMap<String, BTreeMap<String, f64>>, // correlation matrix
    pub cov: BTreeMap<String, BTreeMap<String, f64>>,  // covariance matrix (ann if annualized=true)
    pub returns: BTreeMap<String, Vec<(i64, f64)>>,    // aligned (ts, logret) per symbol
}

/* ------------------------------ Helpers ----------------------------- */

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

fn sample_cov(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 {
        return 0.0;
    }
    let mx = mean(&x[..n]);
    let my = mean(&y[..n]);
    let mut s = 0.0;
    for i in 0..n {
        s += (x[i] - mx) * (y[i] - my);
    }
    s / (n as f64 - 1.0)
}
fn sample_var(x: &[f64]) -> f64 {
    sample_cov(x, x)
}

fn ewma_cov(x: &[f64], y: &[f64], lambda: f64) -> f64 {
    // RiskMetrics-style, zero-mean assumption for daily log returns is fine
    let n = x.len().min(y.len());
    if n == 0 {
        return 0.0;
    }
    let (mut s, mut w, mut weight) = (0.0_f64, 0.0_f64, 1.0_f64);
    for i in (0..n).rev() {
        s += weight * x[i] * y[i];
        w += weight;
        weight *= lambda;
    }
    if w > 0.0 { s / w } else { 0.0 }
}

// trailing dollar-volume EWMA (for decile universe selection)
fn trailing_dv_ewma(bars: &[DailyBar], span: usize) -> f64 {
    if bars.is_empty() {
        return 0.0;
    }
    let alpha = 2.0 / (span as f64 + 1.0);
    let mut s = bars[0].close * bars[0].volume;
    for b in &bars[1..] {
        let dv = b.close * b.volume;
        s = alpha * dv + (1.0 - alpha) * s;
    }
    s
}

/* --------------------------------- API -------------------------------- */

pub async fn get_universe_risk_matrix(
    State(pool): State<PgPool>,
    Query(q): Query<RiskMatrixQuery>,
) -> Json<RiskMatrixOut> {
    // knobs
    let days = q.uni.days.unwrap_or(180).max(30);
    let vol_win = q.uni.vol_window.unwrap_or(30).max(2);
    let min_decile = q.uni.min_decile.unwrap_or(3).clamp(1, 10);
    let method = q.method.clone().unwrap_or_else(|| "sample".to_string());
    let lambda = q.lambda.unwrap_or(0.94).clamp(0.80, 0.999);
    let annualize = q.annualize.unwrap_or(false);

    // fetch bars for entire universe
    let data = match fetch_universe_daily(&pool, &q.uni.exchange, &q.uni.market_type).await {
        Ok(m) => m,
        Err(e) => {
            error!("risk_matrix: fetch_universe_daily failed: {e:?}");
            return Json(RiskMatrixOut {
                method,
                annualized: annualize,
                n_effective: 0,
                symbols: vec![],
                vols: BTreeMap::new(),
                corr: BTreeMap::new(),
                cov: BTreeMap::new(),
                returns: BTreeMap::new(),
            });
        }
    };
    if data.is_empty() {
        return Json(RiskMatrixOut {
            method,
            annualized: annualize,
            n_effective: 0,
            symbols: vec![],
            vols: BTreeMap::new(),
            corr: BTreeMap::new(),
            cov: BTreeMap::new(),
            returns: BTreeMap::new(),
        });
    }

    // universe selection by trailing $volume EWMA deciles
    let mut scored: Vec<(String, f64)> = data
        .iter()
        .map(|(sym, bars)| (sym.clone(), trailing_dv_ewma(bars, vol_win)))
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    let n_total = scored.len().max(1);
    let mut in_universe: HashMap<String, bool> = HashMap::new();
    for (rank, (sym, _)) in scored.into_iter().enumerate() {
        let decile = (10usize.saturating_sub((rank * 10) / n_total)).max(1) as u8;
        in_universe.insert(sym, decile >= min_decile);
    }

    // keep symbols in universe, sorted
    let mut symbols: Vec<String> = data
        .keys()
        .filter(|s| in_universe.get(*s).copied().unwrap_or(false))
        .cloned()
        .collect();
    symbols.sort();

    // intersect timestamps across kept symbols
    let mut ts_inter: Option<BTreeSet<i64>> = None;
    for s in &symbols {
        let set: BTreeSet<i64> = data[s].iter().map(|b| b.ts_ms).collect();
        ts_inter = Some(match ts_inter {
            Some(acc) => acc.intersection(&set).cloned().collect(),
            None => set,
        });
    }
    let ts_inter = ts_inter.unwrap_or_default();
    if ts_inter.len() < 4 {
        info!("risk_matrix: not enough aligned timestamps");
        return Json(RiskMatrixOut {
            method,
            annualized: annualize,
            n_effective: 0,
            symbols,
            vols: BTreeMap::new(),
            corr: BTreeMap::new(),
            cov: BTreeMap::new(),
            returns: BTreeMap::new(),
        });
    }
    let aligned_ts: Vec<i64> = ts_inter.into_iter().collect();

    // aligned close arrays
    let mut price_map: HashMap<String, Vec<f64>> = HashMap::new();
    for s in &symbols {
        let mut v = Vec::with_capacity(aligned_ts.len());
        let rows = &data[s];
        for &ts in &aligned_ts {
            if let Some(b) = rows.iter().find(|b| b.ts_ms == ts) {
                v.push(b.close);
            } else {
                // shouldn't happen due to intersection, but keep shape sane
                v.push(f64::NAN);
            }
        }
        price_map.insert(s.clone(), v);
    }

    // log returns per symbol (drop the seeded first element from log_returns)
    let mut ret_map: HashMap<String, Vec<f64>> = HashMap::new();
    let mut min_len = usize::MAX;
    for s in &symbols {
        let mut lr = log_returns(price_map.get(s).unwrap());
        if !lr.is_empty() {
            lr.remove(0);
        } // drop leading 0.0
        min_len = min_len.min(lr.len());
        ret_map.insert(s.clone(), lr);
    }
    // equalize lengths & sync timestamps to same slice (skip first ts to match returns)
    for s in &symbols {
        ret_map.get_mut(s).unwrap().truncate(min_len);
    }
    let n_eff = min_len;
    if n_eff < 3 {
        return Json(RiskMatrixOut {
            method,
            annualized: annualize,
            n_effective: n_eff,
            symbols,
            vols: BTreeMap::new(),
            corr: BTreeMap::new(),
            cov: BTreeMap::new(),
            returns: BTreeMap::new(),
        });
    }
    let ret_ts: Vec<i64> = aligned_ts
        .iter()
        .copied()
        .skip(aligned_ts.len() - n_eff)
        .collect();

    // replace your current ann_factor line(s) with these:
    let ann_factor: f64 = if annualize { 365.0_f64 } else { 1.0_f64 };
    let ann_sqrt: f64 = if annualize { 365.0_f64.sqrt() } else { 1.0_f64 };

    // store un-annualized vols to compute rho
    let mut unann_vols: HashMap<String, f64> = HashMap::new();
    let mut vols: BTreeMap<String, f64> = BTreeMap::new();
    for s in &symbols {
        let x = ret_map.get(s).unwrap();
        let v = match method.as_str() {
            "ewma" => ewma_cov(x, x, lambda).max(0.0).sqrt(),
            _ => sample_var(x).max(0.0).sqrt(),
        };
        unann_vols.insert(s.clone(), v);
        vols.insert(s.clone(), v * ann_factor.sqrt());
    }

    let mut cov: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    let mut corr: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();

    for i in 0..symbols.len() {
        let si = &symbols[i];
        let xi = ret_map.get(si).unwrap();

        cov.entry(si.clone()).or_insert_with(BTreeMap::new);
        corr.entry(si.clone()).or_insert_with(BTreeMap::new);

        for j in i..symbols.len() {
            let sj = &symbols[j];
            let xj = ret_map.get(sj).unwrap();

            let c = match method.as_str() {
                "ewma" => ewma_cov(xi, xj, lambda),
                _ => sample_cov(xi, xj),
            };
            let c_ann = c * ann_factor;

            let vi = unann_vols[si];
            let vj = unann_vols[sj];
            let denom = (vi * vj).max(1e-12);
            let rho = (c / denom).clamp(-1.0, 1.0);

            cov.get_mut(si).unwrap().insert(sj.clone(), c_ann);
            corr.get_mut(si).unwrap().insert(sj.clone(), rho);

            // mirror
            cov.entry(sj.clone())
                .or_insert_with(BTreeMap::new)
                .insert(si.clone(), c_ann);
            corr.entry(sj.clone())
                .or_insert_with(BTreeMap::new)
                .insert(si.clone(), rho);
        }
    }

    // assemble timestamped returns (aligned)
    let mut returns: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
    for s in &symbols {
        let rs = ret_map.get(s).unwrap();
        let mut pairs = Vec::with_capacity(n_eff);
        for i in 0..n_eff {
            pairs.push((ret_ts[i], rs[i]));
        }
        returns.insert(s.clone(), pairs);
    }

    Json(RiskMatrixOut {
        method,
        annualized: annualize,
        n_effective: n_eff,
        symbols,
        vols,
        corr,
        cov,
        returns,
    })
}
