use axum::{
    Json,
    extract::{Query, State},
};
use sqlx::PgPool;
use std::collections::BTreeMap;

use super::shared::{fetch_single_daily, fetch_universe_daily};
use super::types::{Betas, RegrParams, RegressionReport, TrendPoint, UniverseQuery};

// tiny OLS for small K (normal equations)
fn ols(y: &[f64], xcols: &[Vec<f64>]) -> Option<(f64, Vec<f64>, f64, f64)> {
    let n = y.len();
    if n == 0 {
        return None;
    }
    let k = xcols.len();
    if xcols.iter().any(|c| c.len() != n) {
        return None;
    }

    let cols = k + 1;
    let mut xtx = vec![vec![0.0; cols]; cols];
    let mut xty = vec![0.0; cols];

    for i in 0..n {
        let mut row = vec![1.0];
        for c in xcols {
            row.push(c[i]);
        }
        for a in 0..cols {
            for b in 0..cols {
                xtx[a][b] += row[a] * row[b];
            }
        }
        for a in 0..cols {
            xty[a] += row[a] * y[i];
        }
    }

    // Gaussian elimination
    let mut a = xtx;
    let mut b = xty;
    for p in 0..cols {
        let piv = a[p][p];
        if piv.abs() < 1e-12 {
            return None;
        }
        for j in p..cols {
            a[p][j] /= piv;
        }
        b[p] /= piv;
        for i in 0..cols {
            if i == p {
                continue;
            }
            let f = a[i][p];
            for j in p..cols {
                a[i][j] -= f * a[p][j];
            }
            b[i] -= f * b[p];
        }
    }
    let beta0 = b[0];
    let betas = b[1..].to_vec();

    // R^2 & adj R^2
    let y_mean = y.iter().sum::<f64>() / n as f64;
    let mut sst = 0.0;
    let mut sse = 0.0;
    for i in 0..n {
        let mut yh = beta0;
        for c in 0..k {
            yh += betas[c] * xcols[c][i];
        }
        sst += (y[i] - y_mean).powi(2);
        sse += (y[i] - yh).powi(2);
    }
    let r2 = if sst > 0.0 { 1.0 - sse / sst } else { 0.0 };
    let adj_r2 = if n as i64 - (k as i64) - 1 > 0 {
        1.0 - (1.0 - r2) * ((n as f64 - 1.0) / (n as f64 - k as f64 - 1.0))
    } else {
        r2
    };
    Some((beta0, betas, r2, adj_r2))
}

fn internal<E: std::fmt::Display>(e: E) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
fn bad<S: Into<String>>(s: S) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::BAD_REQUEST, s.into())
}

pub async fn get_xsec_regression(
    State(pool): State<PgPool>,
    Query(q): Query<RegrParams>,
) -> Result<Json<RegressionReport>, (axum::http::StatusCode, String)> {
    // q.days is probably i32 in your RegrParams; enforce minimum and convert as needed
    let days_i32 = q.days.unwrap_or(180).max(30);
    let days_usize: usize = days_i32.try_into().unwrap();

    let horizon = q.horizon.unwrap_or(1).max(1) as usize;

    // fetch universe (function wants i32)
    let data = fetch_universe_daily(&pool, &q.exchange, &q.market_type)
        .await
        .map_err(internal)?;

    // build aligned ts
    let mut all_ts: Vec<i64> = Vec::new();
    for bars in data.values() {
        for b in bars {
            all_ts.push(b.ts_ms);
        }
    }
    all_ts.sort();
    all_ts.dedup();

    // forward returns per symbol at given horizon
    let mut fwd: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
    for (sym, bars) in &data {
        if bars.len() <= horizon {
            continue;
        }
        let mut v = Vec::with_capacity(bars.len() - horizon);
        for i in 0..bars.len() - horizon {
            let r = if bars[i].close != 0.0 {
                (bars[i + horizon].close / bars[i].close) - 1.0
            } else {
                0.0
            };
            v.push((bars[i].ts_ms, r));
        }
        fwd.insert(sym.clone(), v);
    }

    // Synthesize UniverseQuery for signals with sane defaults
    let uq = UniverseQuery {
        exchange: q.exchange.clone(),
        market_type: q.market_type.clone(),
        symbol: q.symbol.clone(),
        days: Some(days_usize),
        vol_window: Some(30),
        min_decile: Some(3),
        timeframe: Some("1d".to_string()),
    };

    // Reuse signals endpoint logic
    let sigs: Vec<TrendPoint> =
        super::signals::cross_section_signals(State(pool.clone()), axum::extract::Query(uq))
            .await
            .0;

    // group signals by ts
    use std::collections::HashMap;
    let mut by_ts: HashMap<i64, Vec<(String, [f64; 4])>> = HashMap::new();
    for r in sigs {
        by_ts
            .entry(r.ts)
            .or_default()
            .push((r.symbol, [r.trend, r.momentum, r.ewmac, r.breakout]));
    }

    // Fama–MacBeth: cross-sectional OLS per date, then average
    let mut betas_sum = [0.0; 4];
    let mut intercept_sum = 0.0;
    let mut r2_sum = 0.0;
    let mut adj_sum = 0.0;
    let mut n_dates = 0usize;
    let mut start_ts = i64::MAX;
    let mut end_ts = i64::MIN;

    for (&ts, rows) in &by_ts {
        let mut y = Vec::new();
        let mut x_tr = Vec::new();
        let mut x_mo = Vec::new();
        let mut x_ew = Vec::new();
        let mut x_bo = Vec::new();

        for (sym, x) in rows {
            if let Some(series) = fwd.get(sym) {
                if let Some((_, ret)) = series.iter().find(|(t, _)| *t == ts) {
                    y.push(*ret);
                    x_tr.push(x[0]);
                    x_mo.push(x[1]);
                    x_ew.push(x[2]);
                    x_bo.push(x[3]);
                }
            }
        }
        if y.len() < 10 {
            continue;
        }

        if let Some((b0, b, r2, adj)) = ols(&y, &[x_tr, x_mo, x_ew, x_bo]) {
            intercept_sum += b0;
            for i in 0..4 {
                betas_sum[i] += b[i];
            }
            r2_sum += r2;
            adj_sum += adj;
            n_dates += 1;
            if ts < start_ts {
                start_ts = ts;
            }
            if ts > end_ts {
                end_ts = ts;
            }
        }
    }

    let report = if n_dates == 0 {
        RegressionReport {
            scope: "xsec".into(),
            exchange: q.exchange,
            market_type: q.market_type,
            horizon_days: horizon as i32,
            start_ts: 0,
            end_ts: 0,
            n: 0,
            intercept: 0.0,
            betas: Betas::default(),
            r2: 0.0,
            adj_r2: 0.0,
        }
    } else {
        let inv = 1.0 / (n_dates as f64);
        RegressionReport {
            scope: "xsec".into(),
            exchange: q.exchange,
            market_type: q.market_type,
            horizon_days: horizon as i32,
            start_ts,
            end_ts,
            n: n_dates,
            intercept: intercept_sum * inv,
            betas: Betas {
                trend: betas_sum[0] * inv,
                momentum: betas_sum[1] * inv,
                ewmac: betas_sum[2] * inv,
                breakout: betas_sum[3] * inv,
            },
            r2: r2_sum * inv,
            adj_r2: adj_sum * inv,
        }
    };

    Ok(Json(report))
}

pub async fn get_ts_regression(
    State(pool): State<PgPool>,
    Query(q): Query<RegrParams>,
) -> Result<Json<RegressionReport>, (axum::http::StatusCode, String)> {
    let symbol = q
        .symbol
        .clone()
        .ok_or_else(|| bad("symbol required for TS regression"))?;

    let days_i32 = q.days.unwrap_or(365).max(60);
    let days_usize: usize = days_i32.try_into().unwrap();
    let horizon = q.horizon.unwrap_or(1).max(1) as usize;

    let bars = fetch_single_daily(&pool, &q.exchange, &q.market_type, &symbol)
        .await
        .map_err(internal)?;

    if bars.len() <= horizon + 120 {
        return Ok(Json(RegressionReport {
            scope: "ts".into(),
            exchange: q.exchange,
            market_type: q.market_type,
            horizon_days: horizon as i32,
            start_ts: 0,
            end_ts: 0,
            n: 0,
            intercept: 0.0,
            betas: Betas::default(),
            r2: 0.0,
            adj_r2: 0.0,
        }));
    }

    // synthesize UniverseQuery defaults
    let uq = UniverseQuery {
        exchange: q.exchange.clone(),
        market_type: q.market_type.clone(),
        symbol: q.symbol.clone(),
        days: Some(days_usize),
        vol_window: Some(30),
        min_decile: Some(3),
        timeframe: Some("1d".to_string()),
    };

    let sigs: Vec<TrendPoint> =
        super::signals::cross_section_signals(State(pool.clone()), axum::extract::Query(uq))
            .await
            .0;

    // forward returns aligned to signal timestamps
    let mut fwd = Vec::<(i64, f64)>::new();
    for i in 0..bars.len().saturating_sub(horizon) {
        let ret = if bars[i].close != 0.0 {
            (bars[i + horizon].close / bars[i].close) - 1.0
        } else {
            0.0
        };
        fwd.push((bars[i].ts_ms, ret));
    }

    let mut y = Vec::new();
    let mut x_tr = Vec::new();
    let mut x_mo = Vec::new();
    let mut x_ew = Vec::new();
    let mut x_bo = Vec::new();
    let mut start_ts = i64::MAX;
    let mut end_ts = i64::MIN;

    for s in sigs {
        if let Some((_, r)) = fwd.iter().find(|(t, _)| *t == s.ts) {
            y.push(*r);
            x_tr.push(s.trend);
            x_mo.push(s.momentum);
            x_ew.push(s.ewmac);
            x_bo.push(s.breakout);
            if s.ts < start_ts {
                start_ts = s.ts;
            }
            if s.ts > end_ts {
                end_ts = s.ts;
            }
        }
    }

    let (beta0, b, r2, adj) = match ols(&y, &[x_tr, x_mo, x_ew, x_bo]) {
        Some(t) => t,
        None => (0.0, vec![0.0; 4], 0.0, 0.0),
    };

    Ok(Json(RegressionReport {
        scope: "ts".into(),
        exchange: q.exchange,
        market_type: q.market_type,
        horizon_days: horizon as i32,
        start_ts: if y.is_empty() { 0 } else { start_ts },
        end_ts: if y.is_empty() { 0 } else { end_ts },
        n: y.len(),
        intercept: beta0,
        betas: Betas {
            trend: b[0],
            momentum: b[1],
            ewmac: b[2],
            breakout: b[3],
        },
        r2,
        adj_r2: adj,
    }))
}
