use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{BTreeMap, HashMap};
use tracing::{error, info};

use crate::math::compute::{breakout, ewmac, momentum};

use super::shared::{DailyBar, fetch_universe_daily};
use super::types::{SignalsCorrQuery, TrendPoint, UniverseQuery};

/* -------------------------- Types -------------------------- */

#[derive(Serialize)]
pub struct CorrOut {
    pub scope: String, // "symbol" or "universe"
    pub n: usize,      // effective paired observations used per entry
    pub matrix: BTreeMap<String, BTreeMap<String, f64>>,
}

/* ---------------------- Small utilities --------------------- */

fn rolling_mean(x: &[f64], window: usize) -> Vec<f64> {
    if window == 0 {
        return vec![0.0; x.len()];
    }
    let mut out = vec![0.0; x.len()];
    let mut sum = 0.0;
    for i in 0..x.len() {
        sum += x[i];
        if i >= window {
            sum -= x[i - window];
        }
        out[i] = sum / (window.min(i + 1) as f64);
    }
    out
}

fn winsorize(v: &mut [f64], p: f64) {
    if v.is_empty() {
        return;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let lo_idx = ((s.len() as f64) * p)
        .floor()
        .clamp(0.0, (s.len() - 1) as f64) as usize;
    let hi_idx = ((s.len() as f64) * (1.0 - p))
        .ceil()
        .clamp(0.0, (s.len() - 1) as f64) as usize;
    let lo = s[lo_idx];
    let hi = s[hi_idx];
    for x in v.iter_mut() {
        if *x < lo {
            *x = lo;
        } else if *x > hi {
            *x = hi;
        }
    }
}

fn mean_sd_inplace(v: &mut [f64]) -> (f64, f64) {
    if v.is_empty() {
        return (0.0, 1.0);
    }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (v.len().max(2) - 1) as f64;
    let sd = var.sqrt().max(1e-12);
    (mean, sd)
}

fn ema_series(x: &[f64], span: usize) -> Vec<f64> {
    if x.is_empty() {
        return vec![];
    }
    let alpha = 2.0 / (span as f64 + 1.0);
    let mut out = Vec::with_capacity(x.len());
    let mut s = x[0];
    out.push(s);
    for &v in &x[1..] {
        s = alpha * v + (1.0 - alpha) * s;
        out.push(s);
    }
    out
}

fn rolling_cov(x: &[f64], y: &[f64], window: usize) -> Vec<Option<f64>> {
    let n = x.len();
    if window == 0 || n == 0 || y.len() != n {
        return vec![None; n];
    }
    let mut out = vec![None; n];
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for i in 0..n {
        let (xi, yi) = (x[i], y[i]);
        sx += xi;
        sy += yi;
        sxx += xi * xi;
        syy += yi * yi;
        sxy += xi * yi;
        if i >= window {
            let (xo, yo) = (x[i - window], y[i - window]);
            sx -= xo;
            sy -= yo;
            sxx -= xo * xo;
            syy -= yo * yo;
            sxy -= xo * yo;
        }
        if i + 1 >= window {
            let w = window as f64;
            let mx = sx / w;
            let my = sy / w;
            let cov = (sxy / w) - mx * my;
            out[i] = Some(cov);
        }
    }
    out
}
fn rolling_var(x: &[f64], window: usize) -> Vec<Option<f64>> {
    rolling_cov(x, x, window)
}

fn mean_opt(xs: &[Option<f64>]) -> Option<f64> {
    let mut s = 0.0;
    let mut c = 0;
    for x in xs {
        if let Some(v) = x {
            s += *v;
            c += 1;
        }
    }
    if c > 0 { Some(s / c as f64) } else { None }
}
fn vec_mean_opt(layers: &[Vec<Option<f64>>]) -> Vec<Option<f64>> {
    if layers.is_empty() {
        return vec![];
    }
    let n = layers[0].len();
    (0..n)
        .map(|i| {
            let row: Vec<Option<f64>> = layers.iter().map(|v| v[i]).collect();
            mean_opt(&row)
        })
        .collect()
}

/* ---------- universe assembly & signals (shared with xsec) ---------- */

fn build_universe_metrics(
    by_symbol: &BTreeMap<String, Vec<DailyBar>>,
    vol_window: usize,
    min_decile: u8,
) -> (
    Vec<i64>,
    HashMap<(i64, String), bool>,
    HashMap<i64, f64>,
    BTreeMap<String, Vec<(i64, f64)>>,
) {
    // Collect dates
    let mut all_dates: Vec<i64> = Vec::new();
    for bars in by_symbol.values() {
        for b in bars {
            all_dates.push(b.ts_ms);
        }
    }
    all_dates.sort();
    all_dates.dedup();

    // trailing $volume per symbol
    let mut trail_dv: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
    for (sym, bars) in by_symbol {
        let dv: Vec<f64> = bars.iter().map(|b| b.close * b.volume).collect();
        let rm = rolling_mean(&dv, vol_window);
        trail_dv.insert(
            sym.clone(),
            bars.iter().zip(rm).map(|(b, v)| (b.ts_ms, v)).collect(),
        );
    }

    // deciles → universe flag
    let mut universe: HashMap<(i64, String), bool> = HashMap::new();
    for &ts in &all_dates {
        let mut vals: Vec<(&str, f64)> = trail_dv
            .iter()
            .filter_map(|(sym, ser)| {
                ser.iter()
                    .find(|(t, _)| *t == ts)
                    .map(|(_, v)| (sym.as_str(), *v))
            })
            .collect();
        if vals.is_empty() {
            continue;
        }
        vals.sort_by(|a, b| b.1.total_cmp(&a.1));
        let n = vals.len();
        for (rank, (sym, _)) in vals.into_iter().enumerate() {
            let decile = (10usize.saturating_sub((rank * 10) / n)).max(1) as u8;
            universe.insert((ts, sym.to_string()), decile >= min_decile);
        }
    }

    // per-symbol log returns & market x-sec avg
    let mut ret_by_symbol: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
    for (sym, bars) in by_symbol {
        let mut r = Vec::with_capacity(bars.len());
        let mut prev: Option<f64> = None;
        for b in bars {
            let lr = match prev {
                Some(p) if p > 0.0 && b.close > 0.0 => (b.close / p).ln(),
                _ => 0.0,
            };
            r.push((b.ts_ms, lr));
            prev = Some(b.close);
        }
        ret_by_symbol.insert(sym.clone(), r);
    }

    let mut ret_mkt_map: HashMap<i64, f64> = HashMap::new();
    for &ts in &all_dates {
        let mut sum = 0.0;
        let mut cnt = 0usize;
        for (sym, series) in &ret_by_symbol {
            if !universe.get(&(ts, sym.clone())).copied().unwrap_or(false) {
                continue;
            }
            if let Some((_, r)) = series.iter().find(|(t, _)| *t == ts) {
                sum += *r;
                cnt += 1;
            }
        }
        if cnt > 0 {
            ret_mkt_map.insert(ts, sum / cnt as f64);
        }
    }

    (all_dates, universe, ret_mkt_map, ret_by_symbol)
}

/* -------------------- Cross-sectional signals -------------------- */

pub async fn cross_section_signals(
    State(pool): State<PgPool>,
    Query(q): Query<UniverseQuery>,
) -> Json<Vec<TrendPoint>> {
    let days_usize = q.days.unwrap_or(90).max(1);
    let days_i32: i32 = days_usize.try_into().unwrap();

    let vol_win = q.vol_window.unwrap_or(30).max(1);
    let min_decile = q.min_decile.unwrap_or(3).clamp(1, 10);

    let data = match fetch_universe_daily(&pool, &q.exchange, &q.market_type).await {
        Ok(m) => m,
        Err(e) => {
            error!("fetch_universe_daily failed: {e:?}");
            return Json(vec![]);
        }
    };
    if data.is_empty() {
        return Json(vec![]);
    }

    let (all_dates, universe, ret_mkt_map, ret_by_symbol) =
        build_universe_metrics(&data, vol_win, min_decile);

    // compute signals per symbol
    let ew_fast = [4usize, 8];
    let brk_win = [20usize, 40, 60];
    let mom_win = [20usize, 30, 60, 90];
    let mom_half_life = 5usize;
    let mom_lag = 1usize;
    let trend_lookback = 20usize;

    struct Raw {
        ts: i64,
        tr: Option<f64>,
        mo: Option<f64>,
        ew: Option<f64>,
        bo: Option<f64>,
    }
    let mut raw_by_symbol: BTreeMap<String, Vec<Raw>> = BTreeMap::new();

    for (sym, bars) in &data {
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let ts_vec: Vec<i64> = bars.iter().map(|b| b.ts_ms).collect();

        // arithmetic returns for momentum
        let mut rets = Vec::with_capacity(close.len());
        if !close.is_empty() {
            rets.push(0.0);
            for i in 1..close.len() {
                let p = close[i - 1];
                rets.push(if p != 0.0 { close[i] / p - 1.0 } else { 0.0 });
            }
        }

        // ewmac / breakout / momentum
        let ewmac_avg = {
            let mut layers: Vec<Vec<Option<f64>>> = Vec::new();
            for &f in &ew_fast {
                layers.push(ewmac(&close, f, Some(4 * f), 25, true, -15.0, 15.0));
            }
            vec_mean_opt(&layers)
        };
        let breakout_avg = {
            let mut layers: Vec<Vec<Option<f64>>> = Vec::new();
            for &w in &brk_win {
                layers.push(breakout(&close, w));
            }
            vec_mean_opt(&layers)
        };
        let momentum_avg = {
            let mut layers: Vec<Vec<Option<f64>>> = Vec::new();
            for &w in &mom_win {
                layers.push(momentum(&rets, w, mom_half_life, mom_lag));
            }
            vec_mean_opt(&layers)
        };

        // trend: β/systematic + idio + mean-reversion
        let mut ret_i: Vec<f64> = Vec::with_capacity(ts_vec.len());
        let mut ret_mkt: Vec<f64> = Vec::with_capacity(ts_vec.len());
        for &ts in &ts_vec {
            let r = ret_by_symbol
                .get(sym.as_str())
                .and_then(|v| v.iter().find(|(t, _)| *t == ts))
                .map(|(_, v)| *v)
                .unwrap_or(0.0);
            let rm = *ret_mkt_map.get(&ts).unwrap_or(&0.0);
            ret_i.push(r);
            ret_mkt.push(rm);
        }
        let cov_im = rolling_cov(&ret_i, &ret_mkt, trend_lookback);
        let var_m = rolling_var(&ret_mkt, trend_lookback);
        let mut beta: Vec<Option<f64>> = vec![None; ts_vec.len()];
        for i in 0..ts_vec.len() {
            if let (Some(c), Some(v)) = (cov_im[i], var_m[i]) {
                beta[i] = Some(if v.abs() > 1e-12 { c / v } else { 0.0 });
            }
        }
        let mut sys_tr: Vec<Option<f64>> = vec![None; ts_vec.len()];
        let mut idio_tr: Vec<Option<f64>> = vec![None; ts_vec.len()];
        for i in 0..ts_vec.len() {
            if let Some(b) = beta[i] {
                let s = b * ret_mkt[i];
                sys_tr[i] = Some(s);
                idio_tr[i] = Some(ret_i[i] - s);
            }
        }
        let mut cum_ret = Vec::with_capacity(ret_i.len());
        let mut acc = 0.0;
        for &v in &ret_i {
            acc += v;
            cum_ret.push(acc);
        }
        let ema_cum = ema_series(&cum_ret, trend_lookback);
        let mut reversion: Vec<Option<f64>> = vec![None; ts_vec.len()];
        for i in 1..ts_vec.len() {
            reversion[i] = Some(-(ema_cum[i] - ema_cum[i - 1]));
        }

        let mut trend_raw: Vec<Option<f64>> = vec![None; ts_vec.len()];
        for i in 0..ts_vec.len() {
            if let (Some(a), Some(b), Some(c)) = (idio_tr[i], sys_tr[i], reversion[i].or(Some(0.0)))
            {
                trend_raw[i] = Some(a + b + c);
            }
        }

        let vec_raw: Vec<Raw> = (0..bars.len())
            .map(|i| Raw {
                ts: bars[i].ts_ms,
                tr: trend_raw[i],
                mo: momentum_avg[i],
                ew: ewmac_avg[i],
                bo: breakout_avg[i],
            })
            .collect();
        raw_by_symbol.insert(sym.clone(), vec_raw);
    }

    // cross-section standardization per date
    let winsor_p = 0.05;
    #[derive(Clone, Copy)]
    struct DailyStats {
        mu_tr: f64,
        sd_tr: f64,
        mu_mo: f64,
        sd_mo: f64,
        mu_ew: f64,
        sd_ew: f64,
        mu_bo: f64,
        sd_bo: f64,
    }
    let mut stats_by_date: HashMap<i64, DailyStats> = HashMap::new();

    // pass 1: collect per-date stats (universe-only)
    for &ts in &all_dates {
        let mut cs_tr = Vec::new();
        let mut cs_mo = Vec::new();
        let mut cs_ew = Vec::new();
        let mut cs_bo = Vec::new();
        for (sym, series) in &raw_by_symbol {
            if !universe.get(&(ts, sym.clone())).copied().unwrap_or(false) {
                continue;
            }
            if let Some(r) = series.iter().find(|r| r.ts == ts) {
                if let (Some(tr), Some(mo), Some(ew), Some(bo)) = (r.tr, r.mo, r.ew, r.bo) {
                    cs_tr.push(tr);
                    cs_mo.push(mo);
                    cs_ew.push(ew);
                    cs_bo.push(bo);
                }
            }
        }
        if cs_tr.len() < 3 {
            continue;
        }
        winsorize(&mut cs_tr, winsor_p);
        winsorize(&mut cs_mo, winsor_p);
        winsorize(&mut cs_ew, winsor_p);
        winsorize(&mut cs_bo, winsor_p);
        let (mu_tr, sd_tr) = mean_sd_inplace(&mut cs_tr);
        let (mu_mo, sd_mo) = mean_sd_inplace(&mut cs_mo);
        let (mu_ew, sd_ew) = mean_sd_inplace(&mut cs_ew);
        let (mu_bo, sd_bo) = mean_sd_inplace(&mut cs_bo);
        stats_by_date.insert(
            ts,
            DailyStats {
                mu_tr,
                sd_tr,
                mu_mo,
                sd_mo,
                mu_ew,
                sd_ew,
                mu_bo,
                sd_bo,
            },
        );
    }

    // pass 2: emit rows
    let weights = (0.30, 0.30, 0.30, 0.10);
    let filter_to = q.symbol.as_ref().map(|s| s.to_ascii_uppercase());

    let mut out: Vec<TrendPoint> = Vec::new();
    for (sym, series) in raw_by_symbol {
        if let Some(ref only) = filter_to {
            if &sym != only {
                continue;
            }
        }
        for r in series {
            if let (Some(stats), Some(tr), Some(mo), Some(ew), Some(bo)) =
                (stats_by_date.get(&r.ts), r.tr, r.mo, r.ew, r.bo)
            {
                let tr_z = (tr - stats.mu_tr) / stats.sd_tr;
                let mo_z = (mo - stats.mu_mo) / stats.sd_mo;
                let ew_z = (ew - stats.mu_ew) / stats.sd_ew;
                let bo_z = (bo - stats.mu_bo) / stats.sd_bo;
                let comp =
                    weights.0 * tr_z + weights.1 * mo_z + weights.2 * ew_z + weights.3 * bo_z;

                out.push(TrendPoint {
                    ts: r.ts,
                    symbol: sym.clone(),
                    trend: tr_z,
                    momentum: mo_z,
                    ewmac: ew_z,
                    breakout: bo_z,
                    composite: comp,
                });
            }
        }
    }
    out.sort_by(|a, b| a.ts.cmp(&b.ts).then(a.symbol.cmp(&b.symbol)));
    info!("trend/xsec built: {} rows", out.len());
    Json(out)
}

/* --------------------- Signals correlation --------------------- */

fn pearson_pair(x: &[f64], y: &[f64]) -> Option<(f64, usize)> {
    // Require at least 3 points
    let n = x.len().min(y.len());
    if n < 3 {
        return None;
    }
    // mean
    let mx = x.iter().sum::<f64>() / n as f64;
    let my = y.iter().sum::<f64>() / n as f64;
    // covariance & variance
    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;
    for i in 0..n {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    if sxx <= 1e-12 || syy <= 1e-12 {
        return Some((0.0, n));
    }
    Some((sxy / (sxx.sqrt() * syy.sqrt()), n))
}
