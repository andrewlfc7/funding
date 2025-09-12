use crate::infra::task_pools::{EndpointPool, threads_from_env};
use std::sync::OnceLock;

use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::{
    fetch_multi_hourly_ohlcv, get_ohlcv_resampled, histogram_counts, log_returns, parse_period_days,
    resample_from_hourly, rolling_mean_std, top_markets_by_usd_volume_live, Tf,
};

fn default_market_type() -> String { "spot".to_string() }
#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }

#[derive(Clone, Debug, Deserialize)]
pub struct VolatilityAnalysisRequest {
    #[serde(default)]
    pub coin: Option<String>,     // optional (single-coin mode if present)
    pub timeframe: String,        // "1h"|"4h"|"1d"
    pub period: String,           // "7d"|"30d"|"90d"|"120d"
    pub exchange: String,         // cex_exchanges.name
    #[serde(default = "default_market_type")]
    pub marketType: String,       // "spot" | "perps"
    #[serde(default)]
    pub topN: Option<i64>,        // universe mode if set
}

#[derive(Debug, Serialize)]
pub struct VolatilityAnalysisResponse {
    pub volatilityTimeSeries: Vec<VRow>,
    pub rangeDistribution: Distribution,
    pub volVsReturns: Vec<VolVsReturns>,
}

#[derive(Debug, Serialize)]
pub struct VRow {
    pub timestamp: i64,
    pub volatility: f64,         // rolling std of log-return
    pub volatilityZScore: f64,   // z(volatility)
    pub returns: f64,            // log-return
    pub high: f64,
    pub low: f64,
    pub range: f64,              // (high-low)/low
    pub symbol: String,          // always set
    pub volume: f64,             // USD notional
}

#[derive(Debug, Serialize)]
pub struct Distribution { pub buckets: Vec<f64>, pub counts: Vec<usize> }

#[derive(Debug, Serialize)]
pub struct VolVsReturns {
    pub volZScore: f64,
    pub returns: f64,
    pub volume: f64,             // USD notional
}

#[derive(Clone)]
struct Job { pool: PgPool, q: VolatilityAnalysisRequest }

static VOL_POOL: OnceLock<EndpointPool<Job, VolatilityAnalysisResponse>> = OnceLock::new();

fn vol_pool() -> &'static EndpointPool<Job, VolatilityAnalysisResponse> {
    VOL_POOL.get_or_init(|| {
        let n = threads_from_env("VOL_ANALYSIS_THREADS", 2);
        EndpointPool::start("vol-analysis", n, |job: Job| async move {
            compute_volatility(job.pool, job.q).await
        })
    })
}

// ========= NEW: compute_volatility (moved heavy logic here) =========

pub async fn compute_volatility(
    pool: PgPool,
    q: VolatilityAnalysisRequest,
) -> VolatilityAnalysisResponse {
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    let days = parse_period_days(&q.period);

    // -------- Universe mode --------
    if let Some(top_n) = q.topN {
        let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();
        let top = match top_markets_by_usd_volume_live(&pool, &q.exchange, &q.marketType, days as i32, top_n).await {
            Ok(v) => v, Err(_) => Vec::new(),
        };
        if top.is_empty() {
            return VolatilityAnalysisResponse {
                volatilityTimeSeries: vec![],
                rangeDistribution: Distribution { buckets: vec![], counts: vec![] },
                volVsReturns: vec![],
            };
        }
        let mids: Vec<i32> = top.iter().map(|(_,mid,_)| *mid).collect();
        let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix).await.unwrap_or_default();

        let mut rows_all: Vec<VRow> = Vec::new();
        let mut scatter: Vec<VolVsReturns> = Vec::new();
        let mut all_ranges: Vec<f64> = Vec::new();

        for (sym, mid, _) in top {
            if let Some(hourly) = by_mid.get(&mid) {
                let series = resample_from_hourly(hourly, tf.period_secs());
                if series.is_empty() { continue; }
                let n = series.len();
                let ts:   Vec<i64> = series.iter().map(|r| r.ts).collect();
                let close:Vec<f64> = series.iter().map(|r| r.close).collect();
                let high: Vec<f64> = series.iter().map(|r| r.high).collect();
                let low:  Vec<f64> = series.iter().map(|r| r.low).collect();
                let base: Vec<f64> = series.iter().map(|r| r.volume).collect();
                let usd_vol: Vec<f64> = close.iter().zip(base.iter()).map(|(p,&v)| finite(p * v)).collect();

                let lr = log_returns(&close);
                let win = tf.steps_per_day().max(6);
                let (_m, s) = rolling_mean_std(&lr, win);
                let win2 = (5 * win).max(6).min(n.max(6));
                let (vm, vs) = rolling_mean_std(&s, win2);
                let vol_z: Vec<f64> = s.iter().enumerate().map(|(i, &v)| {
                    let sd = vs[i];
                    if sd.is_finite() && sd > 0.0 && vm[i].is_finite() { (v - vm[i]) / sd } else { f64::NAN }
                }).collect();

                let start_s  = s.iter().position(|v| v.is_finite()).unwrap_or(n);
                let start_vz = vol_z.iter().position(|v| v.is_finite()).unwrap_or(n);
                let start = start_s.max(start_vz);
                if start >= n { continue; }

                for i in start..n {
                    let rng = if low[i] != 0.0 { (high[i] - low[i]) / low[i] } else { 0.0 };
                    let vz  = finite(vol_z[i]);
                    let ret = finite(lr[i]);
                    let vol = finite(usd_vol[i]);
                    rows_all.push(VRow{
                        timestamp: ts[i],
                        volatility: finite(s[i]),
                        volatilityZScore: vz,
                        returns: ret,
                        high: finite(high[i]),
                        low: finite(low[i]),
                        range: finite(rng),
                        symbol: sym.clone(),
                        volume: vol,
                    });
                    all_ranges.push(finite(rng));
                    if vz.is_finite() && ret.is_finite() {
                        scatter.push(VolVsReturns{ volZScore: vz, returns: ret, volume: vol });
                    }
                }
            }
        }

        let mut buckets = Vec::new(); let mut x = 0.0;
        while x <= 0.20 + 1e-9 { buckets.push(x); x += 0.025; }
        let counts = histogram_counts(&all_ranges, &buckets);

        return VolatilityAnalysisResponse {
            volatilityTimeSeries: rows_all,
            rangeDistribution: Distribution { buckets, counts },
            volVsReturns: scatter,
        };
    }

    // -------- Single-coin mode --------
    let Some(coin) = q.coin.as_deref() else {
        return VolatilityAnalysisResponse {
            volatilityTimeSeries: vec![],
            rangeDistribution: Distribution { buckets: vec![], counts: vec![] },
            volVsReturns: vec![],
        };
    };

    let ohlcv = match get_ohlcv_resampled(&pool, &q.exchange, coin, &q.marketType, tf, days).await {
        Ok(v) => v,
        Err(_) => Vec::new(),
    };
    if ohlcv.is_empty() {
        return VolatilityAnalysisResponse {
            volatilityTimeSeries: vec![],
            rangeDistribution: Distribution { buckets: vec![], counts: vec![] },
            volVsReturns: vec![],
        };
    }

    let n = ohlcv.len();
    let ts:   Vec<i64> = ohlcv.iter().map(|r| r.ts).collect();
    let close:Vec<f64> = ohlcv.iter().map(|r| r.close).collect();
    let high: Vec<f64> = ohlcv.iter().map(|r| r.high).collect();
    let low:  Vec<f64> = ohlcv.iter().map(|r| r.low).collect();
    let base: Vec<f64> = ohlcv.iter().map(|r| r.volume).collect();
    let usd_vol: Vec<f64> = close.iter().zip(base.iter()).map(|(p,&v)| finite(p * v)).collect();

    let lr = log_returns(&close);
    let win = tf.steps_per_day().max(6);
    let (_m, s) = rolling_mean_std(&lr, win);
    let win2 = (5 * win).max(6).min(n.max(6));
    let (vm, vs) = rolling_mean_std(&s, win2);
    let vol_z: Vec<f64> = s.iter().enumerate().map(|(i, &v)| {
        let sd = vs[i];
        if sd.is_finite() && sd > 0.0 && vm[i].is_finite() { (v - vm[i]) / sd } else { f64::NAN }
    }).collect();

    let start_s  = s.iter().position(|v| v.is_finite()).unwrap_or(n);
    let start_vz = vol_z.iter().position(|v| v.is_finite()).unwrap_or(n);
    let start = start_s.max(start_vz);
    if start >= n {
        return VolatilityAnalysisResponse {
            volatilityTimeSeries: vec![],
            rangeDistribution: Distribution { buckets: vec![], counts: vec![] },
            volVsReturns: vec![],
        };
    }

    let mut range = vec![0.0; n];
    for i in 0..n { range[i] = if low[i] != 0.0 { (high[i] - low[i]) / low[i] } else { 0.0 }; }

    let mut buckets = Vec::new(); let mut x = 0.0;
    while x <= 0.20 + 1e-9 { buckets.push(x); x += 0.025; }
    let counts = histogram_counts(&range[start..], &buckets);

    let mut vrows = Vec::with_capacity(n - start);
    let mut scatter = Vec::with_capacity(n - start);
    for i in start..n {
        let vz  = finite(vol_z[i]);
        let ret = finite(lr[i]);
        let vol = finite(usd_vol[i]);
        vrows.push(VRow {
            timestamp: ts[i],
            volatility: finite(s[i]),
            volatilityZScore: vz,
            returns: ret,
            high: finite(high[i]),
            low: finite(low[i]),
            range: finite(range[i]),
            symbol: coin.to_string(),
            volume: vol,
        });
        if vz.is_finite() && ret.is_finite() {
            scatter.push(VolVsReturns{ volZScore: vz, returns: ret, volume: vol });
        }
    }

    VolatilityAnalysisResponse {
        volatilityTimeSeries: vrows,
        rangeDistribution: Distribution { buckets, counts },
        volVsReturns: scatter,
    }
}

// ========= Handler: only posts to the pool =========

pub async fn get_volatility_analysis(
    State(pool): State<PgPool>,
    Query(q): Query<VolatilityAnalysisRequest>,
) -> Json<VolatilityAnalysisResponse> {
    let res = vol_pool().run(Job { pool: pool.clone(), q }).await;
    Json(res)
}
