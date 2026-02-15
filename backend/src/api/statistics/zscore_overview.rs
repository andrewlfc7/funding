use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::OnceLock;

// task-pool infra
use crate::infra::task_pools::{EndpointPool, threads_from_env};

use super::{
    Tf, ewma_span, fetch_multi_hourly_ohlcv, get_ohlcv_resampled, histogram_counts, log_returns,
    parse_period_days, pct_returns, resample_from_hourly, top_markets_by_usd_volume_live,
    zscore_series,
};

fn default_market_type() -> String {
    "spot".to_string()
}

#[inline]
fn finite(x: f64) -> f64 {
    if x.is_finite() { x } else { 0.0 }
}

#[inline]
fn minmax_norm(slice: &[f64]) -> (f64, f64) {
    let mut mn = f64::INFINITY;
    let mut mx = f64::NEG_INFINITY;
    for &v in slice {
        if v.is_finite() {
            if v < mn {
                mn = v;
            }
            if v > mx {
                mx = v;
            }
        }
    }
    if !mn.is_finite() || !mx.is_finite() || mx <= mn {
        (0.0, 1.0)
    } else {
        (mn, mx)
    }
}

#[derive(Debug, Deserialize)]
pub struct ZScoreOverviewRequest {
    #[serde(default)]
    pub baseCoin: Option<String>,
    #[serde(default)]
    pub compareCoin: Option<String>,
    pub timeframe: String,
    pub period: String,
    pub exchange: String,
    #[serde(default = "default_market_type")]
    pub marketType: String,
    #[serde(default)]
    pub topN: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ZScoreOverviewResponse {
    pub zscoreTimeSeries: Vec<ZRow>,
    pub currentZScore: f64,
    pub zscoreDistribution: Distribution,
}

#[derive(Debug, Serialize)]
pub struct ZRow {
    pub timestamp: i64,
    pub price: f64,
    pub zscore: f64,
    pub volume: f64,
    pub returns1h: f64,
    pub returns1d: f64,
    pub logReturns1h: f64,
    pub rollingDollarVolume: f64,
    pub rollingDollarVolumeZ: f64,
    pub rollingDollarVolumeNorm: f64,
    pub symbol: String,
}

#[derive(Debug, Serialize)]
pub struct Distribution {
    pub buckets: Vec<f64>,
    pub counts: Vec<usize>,
}

// ================= Task-pool wiring =================

struct Job {
    pool: PgPool,
    q: ZScoreOverviewRequest,
}

static ZSCORE_POOL: OnceLock<EndpointPool<Job, ZScoreOverviewResponse>> = OnceLock::new();

fn zscore_pool() -> &'static EndpointPool<Job, ZScoreOverviewResponse> {
    ZSCORE_POOL.get_or_init(|| {
        let n = threads_from_env("ZSCORE_THREADS", 2);
        EndpointPool::start("zscore-overview", n, |job: Job| async move {
            compute_zscore_overview(job.pool, job.q).await
        })
    })
}

// ============== Thin handler ==============

pub async fn get_zscore_overview(
    State(db): State<PgPool>,
    Query(q): Query<ZScoreOverviewRequest>,
) -> Json<ZScoreOverviewResponse> {
    let res = zscore_pool()
        .run(Job {
            pool: db.clone(),
            q,
        })
        .await;
    Json(res)
}

// ============== Heavy compute ==============

async fn compute_zscore_overview(pool: PgPool, q: ZScoreOverviewRequest) -> ZScoreOverviewResponse {
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    let days = parse_period_days(&q.period);

    // -------- Universe mode (topN present) --------
    if let Some(top_n) = q.topN {
        let since_unix =
            (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();

        let top = match top_markets_by_usd_volume_live(
            &pool,
            &q.exchange,
            &q.marketType,
            days as i32,
            top_n,
        )
        .await
        {
            Ok(v) => v,
            Err(_) => Vec::new(),
        };

        if top.is_empty() {
            return ZScoreOverviewResponse {
                zscoreTimeSeries: vec![],
                currentZScore: 0.0,
                zscoreDistribution: Distribution {
                    buckets: vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0],
                    counts: vec![0; 7],
                },
            };
        }

        let mids: Vec<i32> = top.iter().map(|(_, mid, _)| *mid).collect();
        let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix)
            .await
            .unwrap_or_default();

        let mut all_rows: Vec<ZRow> = Vec::new();
        let mut all_z: Vec<f64> = Vec::new();
        let mut last_z_sum = 0.0_f64;
        let mut last_z_cnt: usize = 0;

        for (sym, mid, _) in top {
            let Some(hourly) = by_mid.get(&mid) else {
                continue;
            };
            let series = resample_from_hourly(hourly, tf.period_secs());
            if series.is_empty() {
                continue;
            }

            let n = series.len();
            let ts: Vec<i64> = series.iter().map(|r| r.ts).collect();
            let price: Vec<f64> = series.iter().map(|r| r.close).collect();
            let base_v: Vec<f64> = series.iter().map(|r| r.volume).collect();

            // USD notional per bar
            let usd_v: Vec<f64> = price
                .iter()
                .zip(base_v.iter())
                .map(|(p, &v)| finite(p * v))
                .collect();

            // Windows
            let win = (n / 6).clamp(24, 240);

            // Metrics
            let z = zscore_series(&price, win);
            let rets = pct_returns(&price);
            let log_rets = log_returns(&price);

            // returns over one "day" in the chosen TF
            let daily_steps = match tf {
                Tf::H1 => 24,
                Tf::H4 => 6,
                Tf::D1 => 1,
            };
            let mut returns1d = vec![0.0; n];
            for i in daily_steps..n {
                let p0 = price[i - daily_steps];
                returns1d[i] = finite(if p0 != 0.0 {
                    (price[i] / p0) - 1.0
                } else {
                    0.0
                });
            }

            // EWMA(USD vol) and its z-score
            let vol_ewma = ewma_span(&usd_v, win.max(24));
            let vol_ewma_z = zscore_series(&vol_ewma, win);

            // First mature index
            let start_price_z = z.iter().position(|v| v.is_finite()).unwrap_or(n);
            let start_vol_z = vol_ewma_z.iter().position(|v| v.is_finite()).unwrap_or(n);
            let start = start_price_z.max(start_vol_z);
            if start >= n {
                continue;
            }

            // Normalize EWMA(USD vol) on matured slice
            let (mn, mx) = minmax_norm(&vol_ewma[start..]);
            let denom = (mx - mn).max(1e-12);

            for i in start..n {
                let zi = finite(z[i]);
                all_rows.push(ZRow {
                    timestamp: ts[i],
                    price: finite(price[i]),
                    zscore: zi,
                    volume: finite(usd_v[i]),
                    returns1h: finite(rets[i]),
                    returns1d: finite(returns1d[i]),
                    logReturns1h: finite(log_rets[i]),
                    rollingDollarVolume: finite(vol_ewma[i]),
                    rollingDollarVolumeZ: finite(vol_ewma_z[i]),
                    rollingDollarVolumeNorm: finite((vol_ewma[i] - mn) / denom),
                    symbol: sym.clone(),
                });
                all_z.push(zi);
            }

            if let Some(&lz) = z[start..].last() {
                if lz.is_finite() {
                    last_z_sum += lz;
                    last_z_cnt += 1;
                }
            }
        }

        // Distribution & current
        let buckets = vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0];
        let counts = histogram_counts(
            &all_z
                .iter()
                .copied()
                .filter(|x| x.is_finite())
                .collect::<Vec<_>>(),
            &buckets,
        );
        let current_z = if last_z_cnt > 0 {
            last_z_sum / (last_z_cnt as f64)
        } else {
            0.0
        };

        return ZScoreOverviewResponse {
            zscoreTimeSeries: all_rows,
            currentZScore: finite(current_z),
            zscoreDistribution: Distribution { buckets, counts },
        };
    }

    // -------- Single-coin mode --------
    let Some(base) = q.baseCoin.as_deref() else {
        return ZScoreOverviewResponse {
            zscoreTimeSeries: vec![],
            currentZScore: 0.0,
            zscoreDistribution: Distribution {
                buckets: vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0],
                counts: vec![0; 7],
            },
        };
    };

    let ohlcv = match get_ohlcv_resampled(&pool, &q.exchange, base, &q.marketType, tf, days).await {
        Ok(v) => v,
        Err(_) => Vec::new(),
    };
    if ohlcv.is_empty() {
        return ZScoreOverviewResponse {
            zscoreTimeSeries: vec![],
            currentZScore: 0.0,
            zscoreDistribution: Distribution {
                buckets: vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0],
                counts: vec![0; 7],
            },
        };
    }

    let n = ohlcv.len();
    let ts: Vec<i64> = ohlcv.iter().map(|r| r.ts).collect();
    let price: Vec<f64> = ohlcv.iter().map(|r| r.close).collect();
    let base_v: Vec<f64> = ohlcv.iter().map(|r| r.volume).collect();
    let usd_v: Vec<f64> = price
        .iter()
        .zip(base_v.iter())
        .map(|(p, &v)| finite(p * v))
        .collect();

    let win = (n / 6).clamp(24, 240);
    let z = zscore_series(&price, win);
    let rets = pct_returns(&price);
    let log_rets = log_returns(&price);

    let daily_steps = match tf {
        Tf::H1 => 24,
        Tf::H4 => 6,
        Tf::D1 => 1,
    };
    let mut returns1d = vec![0.0; n];
    for i in daily_steps..n {
        let p0 = price[i - daily_steps];
        returns1d[i] = finite(if p0 != 0.0 {
            (price[i] / p0) - 1.0
        } else {
            0.0
        });
    }

    let vol_ewma = ewma_span(&usd_v, win.max(24));
    let vol_ewma_z = zscore_series(&vol_ewma, win);

    // drop warm-up rows
    let start_price_z = z.iter().position(|v| v.is_finite()).unwrap_or(n);
    let start_volz = vol_ewma_z.iter().position(|v| v.is_finite()).unwrap_or(n);
    let start = start_price_z.max(start_volz);
    if start >= n {
        return ZScoreOverviewResponse {
            zscoreTimeSeries: vec![],
            currentZScore: 0.0,
            zscoreDistribution: Distribution {
                buckets: vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0],
                counts: vec![0; 7],
            },
        };
    }

    // min–max normalize EWMA USD volume on matured slice
    let (mn, mx) = minmax_norm(&vol_ewma[start..]);
    let denom = (mx - mn).max(1e-12);

    // build rows
    let mut rows = Vec::with_capacity(n - start);
    for i in start..n {
        rows.push(ZRow {
            timestamp: ts[i],
            price: finite(price[i]),
            zscore: finite(z[i]),
            volume: finite(usd_v[i]),
            returns1h: finite(rets[i]),
            returns1d: finite(returns1d[i]),
            logReturns1h: finite(log_rets[i]),
            rollingDollarVolume: finite(vol_ewma[i]),
            rollingDollarVolumeZ: finite(vol_ewma_z[i]),
            rollingDollarVolumeNorm: finite((vol_ewma[i] - mn) / denom),
            symbol: base.to_string(),
        });
    }

    // last finite z after warm-up
    let current_z = z
        .iter()
        .skip(start)
        .rev()
        .find(|v| v.is_finite())
        .copied()
        .unwrap_or(0.0);

    ZScoreOverviewResponse {
        zscoreTimeSeries: rows,
        currentZScore: finite(current_z),
        zscoreDistribution: {
            let buckets = vec![-3.0, -2.0, -1.0, 0.0, 1.0, 2.0, 3.0];
            let counts = histogram_counts(
                &z[start..]
                    .iter()
                    .copied()
                    .filter(|x| x.is_finite())
                    .collect::<Vec<_>>(),
                &buckets,
            );
            Distribution { buckets, counts }
        },
    }
}
