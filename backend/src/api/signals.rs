use axum::{
    extract::{Query, State},
    response::Json,
    routing::get,
    Router,
};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::{BTreeMap, HashMap};
use tracing::{error, info};

use crate::math::compute::{breakout, ewmac, momentum};

#[derive(Debug, Deserialize)]
pub struct XSecQuery {
    pub exchange: String,           // "binance"
    pub market_type: String,        // "spot" | "perps"
    pub days: Option<i32>,          // default 180
    pub vol_window: Option<usize>,  // default 30
    pub min_decile: Option<u8>,     // default 3
    pub symbol: Option<String>,     // optional BASE symbol (e.g., BTC). If omitted, return all symbols.
}

#[derive(Debug, Serialize)]
pub struct XSecSignalRow {
    pub ts: i64,        // epoch ms
    pub symbol: String, // base symbol
    pub trend: f64,     // z-scores vs. cross-section at ts
    pub momentum: f64,
    pub ewmac: f64,
    pub breakout: f64,
    pub composite: f64,
}

/* ---------------------- helpers ---------------------- */

fn rolling_mean(x: &[f64], window: usize) -> Vec<f64> {
    if window == 0 { return vec![0.0; x.len()]; }
    let mut out = vec![0.0; x.len()];
    let mut sum = 0.0;
    for i in 0..x.len() {
        sum += x[i];
        if i >= window { sum -= x[i - window]; }
        out[i] = sum / (window.min(i + 1) as f64);
    }
    out
}

fn winsorize(v: &mut [f64], p: f64) {
    if v.is_empty() { return; }
    let mut s = v.to_vec();
    s.sort_by(|a,b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo_idx = ((s.len() as f64) * p).floor().clamp(0.0, (s.len()-1) as f64) as usize;
    let hi_idx = ((s.len() as f64) * (1.0 - p)).ceil().clamp(0.0, (s.len()-1) as f64) as usize;
    let lo = s[lo_idx];
    let hi = s[hi_idx];
    for x in v.iter_mut() {
        if *x < lo { *x = lo; } else if *x > hi { *x = hi; }
    }
}

fn mean_sd_inplace(v: &mut [f64]) -> (f64, f64) {
    if v.is_empty() { return (0.0, 1.0); }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x-mean)*(x-mean)).sum::<f64>() / (v.len().max(2)-1) as f64;
    let sd = var.sqrt().max(1e-12);
    (mean, sd)
}

/// EMA series with span (alpha = 2/(span+1)), seeded with first value.
fn ema_series(x: &[f64], span: usize) -> Vec<f64> {
    if x.is_empty() { return vec![]; }
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

/// Rolling covariance via running sums.
fn rolling_cov(x: &[f64], y: &[f64], window: usize) -> Vec<Option<f64>> {
    let n = x.len();
    if window == 0 || n == 0 || y.len() != n { return vec![None; n]; }
    let mut out = vec![None; n];
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for i in 0..n {
        let (xi, yi) = (x[i], y[i]);
        sx += xi; sy += yi; sxx += xi*xi; syy += yi*yi; sxy += xi*yi;
        if i >= window {
            let (xo, yo) = (x[i - window], y[i - window]);
            sx -= xo; sy -= yo; sxx -= xo*xo; syy -= yo*yo; sxy -= xo*yo;
        }
        if i + 1 >= window {
            let w = window as f64;
            let mx = sx / w; let my = sy / w;
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
        if let Some(v) = x { s += *v; c += 1; }
    }
    if c > 0 { Some(s / c as f64) } else { None }
}

fn vec_mean_opt(layers: &[Vec<Option<f64>>]) -> Vec<Option<f64>> {
    if layers.is_empty() { return vec![]; }
    let n = layers[0].len();
    (0..n)
        .map(|i| {
            let row: Vec<Option<f64>> = layers.iter().map(|v| v[i]).collect();
            mean_opt(&row)
        })
        .collect()
}

/* ---------------------- handler ---------------------- */

pub async fn cross_section_signals(
    State(pool): State<PgPool>,
    Query(q): Query<XSecQuery>,
) -> Json<Vec<XSecSignalRow>> {
    let days        = q.days.unwrap_or(180).max(1);
    let vol_window  = q.vol_window.unwrap_or(30).max(1);
    let min_decile  = q.min_decile.unwrap_or(3).clamp(1, 10);
    let mt          = q.market_type.to_lowercase();
    let ex          = q.exchange.to_lowercase();
    let want_sym    = q.symbol.as_ref().map(|s| s.to_ascii_uppercase());

    // Resolve exchange_id
    let ex_row = sqlx::query("SELECT id FROM cex_exchanges WHERE lower(name) = $1 LIMIT 1")
        .bind(&ex)
        .fetch_optional(&pool)
        .await;

    let exchange_id: i32 = match ex_row {
        Ok(Some(r)) => r.get::<i32, _>("id"),
        _ => {
            error!("exchange not found: {}", ex);
            return Json(vec![]);
        }
    };

    // Pull markets for this exchange/type; use base symbol m.symbol
    let rows = sqlx::query!(
        r#"
        SELECT
          m.symbol        AS "base_symbol!",
          k.date          AS "date!",
          k."close"::double precision  AS "close!",
          k.high::double precision     AS "high!",
          k.low::double precision      AS "low!",
          k.volume::double precision   AS "volume!"
        FROM klines_daily k
        JOIN cex_markets m ON m.id = k.market_id
        WHERE m.exchange_id = $1
          AND m.market_type = $2
          AND m.is_active = TRUE
          AND k.date >= now() - ($3::int * interval '1 day')
        ORDER BY k.date ASC, m.symbol ASC
        "#,
        exchange_id,
        &mt,
        days
    )
    .fetch_all(&pool)
    .await;

    let rows = match rows {
        Ok(rs) => rs,
        Err(e) => {
            error!("klines fetch failed: {:?}", e);
            return Json(vec![]);
        }
    };

    #[derive(Clone)]
    struct Bar { ts: i64, close: f64, high: f64, low: f64, vol: f64 }

    let mut by_symbol: BTreeMap<String, Vec<Bar>> = BTreeMap::new();
    let mut all_dates: Vec<i64> = Vec::new();

    for r in rows {
        let offset_datetime = r.date.midnight().assume_utc();
        let ts = offset_datetime.unix_timestamp() * 1000;

        let sym = r.base_symbol.to_ascii_uppercase();
        by_symbol.entry(sym).or_default().push(Bar {
            ts,
            close: r.close,
            high: r.high,
            low: r.low,
            vol: r.volume,
        });
        all_dates.push(ts);
    }
    all_dates.sort();
    all_dates.dedup();

    /* -------- Volume filter: rolling dollar volume & per-date deciles -------- */
    let mut trail_dv: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
    for (sym, bars) in &by_symbol {
        let dv: Vec<f64> = bars.iter().map(|b| b.close * b.vol).collect();
        let rm = rolling_mean(&dv, vol_window);
        trail_dv.insert(sym.clone(), bars.iter().zip(rm).map(|(b, v)| (b.ts, v)).collect());
    }

    let mut universe: HashMap<(i64, &str), bool> = HashMap::new();
    for &ts in &all_dates {
        let mut vals: Vec<(&str, f64)> = trail_dv
            .iter()
            .filter_map(|(sym, series)| {
                series.iter().find(|(t, _)| *t == ts).map(|(_, v)| (sym.as_str(), *v))
            })
            .collect();
        if vals.is_empty() { continue; }
        vals.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let n = vals.len();
        for (rank, (sym, _)) in vals.into_iter().enumerate() {
            let decile = (10usize.saturating_sub((rank * 10) / n)).max(1) as u8;
            universe.insert((ts, sym), decile >= min_decile);
        }
    }

    /* -------- Per-symbol log-returns & market (x-sec mean) per date -------- */
    let mut ret_by_symbol: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
    for (sym, bars) in &by_symbol {
        let mut r = Vec::with_capacity(bars.len());
        let mut prev: Option<f64> = None;
        for b in bars {
            let lr = match prev {
                Some(p) if p > 0.0 && b.close > 0.0 => b.close.ln() - p.ln(),
                _ => 0.0,
            };
            r.push((b.ts, lr));
            prev = Some(b.close);
        }
        ret_by_symbol.insert(sym.clone(), r);
    }

    let mut ret_mkt_map: HashMap<i64, f64> = HashMap::new();
    for &ts in &all_dates {
        let mut sum = 0.0;
        let mut cnt = 0usize;
        for (sym, series) in &ret_by_symbol {
            if !universe.get(&(ts, sym.as_str())).copied().unwrap_or(false) { continue; }
            if let Some((_, r)) = series.iter().find(|(t, _)| *t == ts) {
                sum += *r;
                cnt += 1;
            }
        }
        if cnt > 0 {
            ret_mkt_map.insert(ts, sum / cnt as f64);
        }
    }

    /* -------- Raw signals per symbol, avg across windows -------- */
    struct Raw {
        ts: i64,
        tr: Option<f64>,
        mo: Option<f64>,
        ew: Option<f64>,
        bo: Option<f64>,
    }

    let ew_fast = [4usize, 8, 16];    // EWMAC uses (f, 4f)
    let brk_win = [20usize, 40, 60];
    let mom_win = [20usize, 30, 60, 90, 120];
    let mom_half_life = 5usize;
    let mom_lag = 1usize;
    let trend_lookback = 60usize;

    let mut raw_by_symbol: BTreeMap<String, Vec<Raw>> = BTreeMap::new();

    for (sym, bars) in &by_symbol {
        let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
        let ts_vec: Vec<i64> = bars.iter().map(|b| b.ts).collect();

        // arithmetic returns for momentum
        let mut rets = Vec::with_capacity(close.len());
        if !close.is_empty() {
            rets.push(0.0);
            for i in 1..close.len() {
                let p = close[i - 1];
                rets.push(if p != 0.0 { close[i] / p - 1.0 } else { 0.0 });
            }
        }

        // EWMAC avg across (f, 4f)
        let mut ew_layers: Vec<Vec<Option<f64>>> = Vec::new();
        for &f in &ew_fast {
            ew_layers.push(ewmac(&close, f, Some(4 * f), 25, true, -15.0, 15.0));
        }
        let ewmac_avg = vec_mean_opt(&ew_layers);

        // Breakout avg across windows
        let mut bo_layers: Vec<Vec<Option<f64>>> = Vec::new();
        for &w in &brk_win {
            bo_layers.push(breakout(&close, w));
        }
        let breakout_avg = vec_mean_opt(&bo_layers);

        // Momentum avg across windows
        let mut mo_layers: Vec<Vec<Option<f64>>> = Vec::new();
        for &w in &mom_win {
            mo_layers.push(momentum(&rets, w, mom_half_life, mom_lag));
        }
        let momentum_avg = vec_mean_opt(&mo_layers);

        // Trend (β/systematic + idio + mean-reversion)
        // Align own returns & market returns
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

        // β = Cov(ret_i, ret_mkt)/Var(ret_mkt)
        let cov_im = rolling_cov(&ret_i, &ret_mkt, trend_lookback);
        let var_m = rolling_var(&ret_mkt, trend_lookback);
        let mut beta: Vec<Option<f64>> = vec![None; ts_vec.len()];
        for i in 0..ts_vec.len() {
            if let (Some(c), Some(v)) = (cov_im[i], var_m[i]) {
                beta[i] = Some(if v.abs() > 1e-12 { c / v } else { 0.0 });
            }
        }

        // sys_tr = β * ret_mkt ; idio_tr = ret_i - sys_tr
        let mut sys_tr: Vec<Option<f64>> = vec![None; ts_vec.len()];
        let mut idio_tr: Vec<Option<f64>> = vec![None; ts_vec.len()];
        for i in 0..ts_vec.len() {
            if let Some(b) = beta[i] {
                let s = b * ret_mkt[i];
                sys_tr[i] = Some(s);
                idio_tr[i] = Some(ret_i[i] - s);
            }
        }

        // reversion = -(EMA(cum_ret, span=L) - lag1)
        let mut cum_ret = Vec::with_capacity(ret_i.len());
        let mut acc = 0.0;
        for &v in &ret_i { acc += v; cum_ret.push(acc); }
        let ema_cum = ema_series(&cum_ret, trend_lookback);
        let mut reversion: Vec<Option<f64>> = vec![None; ts_vec.len()];
        for i in 1..ts_vec.len() {
            reversion[i] = Some(-(ema_cum[i] - ema_cum[i - 1]));
        }

        let mut trend_raw: Vec<Option<f64>> = vec![None; ts_vec.len()];
        for i in 0..ts_vec.len() {
            if let (Some(a), Some(b), Some(c)) = (idio_tr[i], sys_tr[i], reversion[i].or(Some(0.0))) {
                trend_raw[i] = Some(a + b + c);
            }
        }

        let vec_raw: Vec<Raw> = (0..bars.len())
            .map(|i| Raw {
                ts: bars[i].ts,
                tr: trend_raw[i],
                mo: momentum_avg[i],
                ew: ewmac_avg[i],
                bo: breakout_avg[i],
            })
            .collect();
        raw_by_symbol.insert(sym.clone(), vec_raw);
    }

    /* -------- Cross-section standardization per date -------- */
    let winsor_p = 0.05;
    let (w_trend, w_mom, w_ew, w_bo) = (0.30, 0.30, 0.30, 0.10);
    
    #[derive(Clone, Copy)]
    struct DailyStats {
        mu_tr: f64, sd_tr: f64,
        mu_mo: f64, sd_mo: f64,
        mu_ew: f64, sd_ew: f64,
        mu_bo: f64, sd_bo: f64,
    }
    let mut stats_by_date: HashMap<i64, DailyStats> = HashMap::new();

    // PASS 1
    for &ts in &all_dates {
        let mut cs_tr: Vec<f64> = Vec::new();
        let mut cs_mo: Vec<f64> = Vec::new();
        let mut cs_ew: Vec<f64> = Vec::new();
        let mut cs_bo: Vec<f64> = Vec::new();

        for (sym, series) in &raw_by_symbol {
            if !universe.get(&(ts, sym.as_str())).copied().unwrap_or(false) { continue; }
            if let Some(r) = series.iter().find(|r| r.ts == ts) {
                if let (Some(tr), Some(mo), Some(ew), Some(bo)) = (r.tr, r.mo, r.ew, r.bo) {
                    cs_tr.push(tr);
                    cs_mo.push(mo);
                    cs_ew.push(ew);
                    cs_bo.push(bo);
                }
            }
        }
        if cs_tr.len() < 3 { continue; }

        winsorize(&mut cs_tr, winsor_p);
        winsorize(&mut cs_mo, winsor_p);
        winsorize(&mut cs_ew, winsor_p);
        winsorize(&mut cs_bo, winsor_p);
        let (mu_tr, sd_tr) = mean_sd_inplace(&mut cs_tr);
        let (mu_mo, sd_mo) = mean_sd_inplace(&mut cs_mo);
        let (mu_ew, sd_ew) = mean_sd_inplace(&mut cs_ew);
        let (mu_bo, sd_bo) = mean_sd_inplace(&mut cs_bo);

        stats_by_date.insert(ts, DailyStats { mu_tr, sd_tr, mu_mo, sd_mo, mu_ew, sd_ew, mu_bo, sd_bo });
    }

    // PASS 2
    let all_syms_uc: Vec<String> = by_symbol.keys().cloned().collect();
    let emit_syms: Vec<String> = match &want_sym {
        Some(s) => vec![s.clone()],
        None => all_syms_uc,
    };
    let mut out: Vec<XSecSignalRow> = Vec::new();

    for sym_name in &emit_syms {
        if let Some(raw_series) = raw_by_symbol.get(sym_name) {
            for r in raw_series {
                if let (Some(stats), Some(tr), Some(mo), Some(ew), Some(bo)) =
                    (stats_by_date.get(&r.ts), r.tr, r.mo, r.ew, r.bo)
                {
                    let tr_z = (tr - stats.mu_tr) / stats.sd_tr;
                    let mo_z = (mo - stats.mu_mo) / stats.sd_mo;
                    let ew_z = (ew - stats.mu_ew) / stats.sd_ew;
                    let bo_z = (bo - stats.mu_bo) / stats.sd_bo;
                    let comp = w_trend * tr_z + w_mom * mo_z + w_ew * ew_z + w_bo * bo_z;

                    out.push(XSecSignalRow {
                        ts: r.ts,
                        symbol: sym_name.clone(),
                        trend: tr_z,
                        momentum: mo_z,
                        ewmac: ew_z,
                        breakout: bo_z,
                        composite: comp,
                    });
                }
            }
        }
    }

    out.sort_by(|a, b| a.ts.cmp(&b.ts).then(a.symbol.cmp(&b.symbol)));

    // persist (optional; unchanged)
    if !out.is_empty() {
        if let Ok(mut tx) = pool.begin().await {
            let mut transaction_failed = false;
            let query = r#"
            INSERT INTO xsec_signals (ts, symbol, exchange, market_type, trend, momentum, ewmac, breakout, composite)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (ts, symbol, exchange, market_type) DO UPDATE SET
                trend = EXCLUDED.trend,
                momentum = EXCLUDED.momentum,
                ewmac = EXCLUDED.ewmac,
                breakout = EXCLUDED.breakout,
                composite = EXCLUDED.composite;
            "#;

            for row in &out {
                if let Some(ts_chrono) = DateTime::from_timestamp_millis(row.ts) {
                    if let Err(e) = sqlx::query(query)
                        .bind(ts_chrono)
                        .bind(&row.symbol)
                        .bind(&ex)
                        .bind(&mt)
                        .bind(row.trend)
                        .bind(row.momentum)
                        .bind(row.ewmac)
                        .bind(row.breakout)
                        .bind(row.composite)
                        .execute(&mut *tx)
                        .await
                    {
                        error!("failed to upsert xsec_signal row: {:?}, marking for rollback.", e);
                        transaction_failed = true;
                        break;
                    }
                }
            }
            
            if transaction_failed {
                if let Err(e) = tx.rollback().await {
                    error!("failed to rollback transaction for xsec_signals: {:?}", e);
                }
            } else {
                if let Err(e) = tx.commit().await {
                    error!("failed to commit transaction for xsec_signals: {:?}", e);
                }
            }
        } else {
            error!("failed to begin transaction to save xsec_signals");
        }
    }

    info!("xsec signals built: {} rows (emit_syms={})", out.len(), emit_syms.len());
    Json(out)
}

/* ---------------------- router ---------------------- */

pub fn router() -> Router<PgPool> {
    Router::<PgPool>::new()
        .route("/api/signals/xsec", get(cross_section_signals))
}
