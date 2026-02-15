use axum::{
    Json,
    extract::{Query, State},
};
use serde::de::{Error as DeError, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::PgPool;
use std::{collections::BTreeSet, fmt};

use super::{
    Tf, fetch_multi_hourly_ohlcv, parse_period_days, resample_from_hourly,
    top_markets_by_usd_volume_live, zscore_series,
};

// NEW: task-pool infra
use crate::infra::task_pools::{EndpointPool, threads_from_env};
use std::sync::OnceLock;

fn default_market_type() -> String {
    "spot".to_string()
}
fn default_timeframe() -> String {
    "1h".to_string()
}
fn default_window() -> usize {
    48
} // z-score lookback
fn default_vel_win() -> usize {
    6
} // velocity window (in TF steps)
#[inline]
fn finite(x: f64) -> f64 {
    if x.is_finite() { x } else { 0.0 }
}

fn string_or_seq<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StrOrSeq;
    impl<'de> Visitor<'de> for StrOrSeq {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "string/CSV/sequence")
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: DeError,
        {
            Ok(v.split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect())
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut out = Vec::new();
            while let Some(elem) = seq.next_element::<String>()? {
                out.extend(
                    elem.split(',')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .map(String::from),
                );
            }
            Ok(out)
        }
    }
    de.deserialize_any(StrOrSeq)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeMomentumRequest {
    pub period: String,
    pub exchange: String,
    #[serde(default = "default_market_type")]
    pub market_type: String,
    #[serde(default)]
    pub top_n: Option<i64>,
    #[serde(default)]
    pub coins: Option<Vec<String>>,
    #[serde(default = "default_window")]
    pub window: usize,
    #[serde(default = "default_vel_win")]
    pub velocity_window: usize,
    /// timeframes like ["1h","4h","1d","3d","7d"]
    #[serde(default = "default_tfs", deserialize_with = "string_or_seq")]
    pub timeframes: Vec<String>,
}
fn default_tfs() -> Vec<String> {
    vec!["1h".into(), "4h".into(), "1d".into()]
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeMomentumResponse {
    pub heatmap: Heatmap, // coins x timeframes (Δz over scaled velocity window)
    pub transition_matrix: [[f64; 3]; 3], // bull/bear/range transition probs (row-normalized)
    pub velocity_series: Vec<VelSeries>, // per-coin series of {timestamp, velocity, acceleration}
    pub cross_asset_divergence: DivergenceSnapshot,
}

#[derive(Debug, Serialize)]
pub struct Heatmap {
    pub coins: Vec<String>,
    pub timeframes: Vec<String>,
    pub matrix: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct VelPoint {
    pub timestamp: i64,
    pub velocity: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceleration: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct VelSeries {
    pub symbol: String,
    pub series: Vec<VelPoint>,
}

#[derive(Debug, Serialize)]
pub struct DivergenceSnapshot {
    pub leaders: Vec<String>,
    pub laggards: Vec<String>,
    pub score: f64,
}

fn parse_tf_and_mult(s: &str) -> (Tf, usize) {
    match s.to_ascii_lowercase().as_str() {
        "1h" => (Tf::H1, 1),
        "4h" => (Tf::H4, 1),
        "1d" | "d1" => (Tf::D1, 1),
        "3d" => (Tf::D1, 3),
        "7d" => (Tf::D1, 7),
        _ => (Tf::H1, 1),
    }
}

#[inline]
fn regime_state_from_z(z: f64) -> usize {
    // 0 = Bull, 1 = Bear, 2 = Range
    if z > 0.5 {
        0
    } else if z < -0.5 {
        1
    } else {
        2
    }
}

// ---------- Task pool wiring ----------

struct Job {
    pool: PgPool,
    q: RegimeMomentumRequest,
}

static REGIME_POOL: OnceLock<EndpointPool<Job, RegimeMomentumResponse>> = OnceLock::new();

fn pool() -> &'static EndpointPool<Job, RegimeMomentumResponse> {
    REGIME_POOL.get_or_init(|| {
        let n = threads_from_env("REGIME_THREADS", 2);
        EndpointPool::start("regime-momentum", n, |job: Job| async move {
            compute_regime_momentum(job.pool, job.q).await
        })
    })
}

// Lightweight handler -> dispatch to pool
pub async fn get_regime_momentum(
    State(db): State<PgPool>,
    Query(q): Query<RegimeMomentumRequest>,
) -> Json<RegimeMomentumResponse> {
    let res = pool()
        .run(Job {
            pool: db.clone(),
            q,
        })
        .await;
    Json(res)
}

// ---------- Heavy compute ----------

async fn compute_regime_momentum(pool: PgPool, q: RegimeMomentumRequest) -> RegimeMomentumResponse {
    let days = parse_period_days(&q.period);
    let since_unix =
        (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();

    // Universe: explicit + topN
    let mut syms: BTreeSet<String> = q
        .coins
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.to_uppercase())
        .collect();
    if let Some(n) = q.top_n {
        if let Ok(top) =
            top_markets_by_usd_volume_live(&pool, &q.exchange, &q.market_type, days as i32, n).await
        {
            syms.extend(top.into_iter().map(|(s, _, _)| s.to_uppercase()));
        }
    }
    let mut all_syms: Vec<String> = syms.into_iter().collect();
    all_syms.sort_unstable();

    // Resolve + fetch hourly once; we’ll resample per TF
    let mut mids = Vec::new();
    let mut sym_mid = Vec::new();
    for s in &all_syms {
        if let Ok(mid) =
            super::resolve_market_id_with_data(&pool, &q.exchange, s, &q.market_type, since_unix)
                .await
        {
            mids.push(mid);
            sym_mid.push((s.clone(), mid));
        }
    }
    let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix)
        .await
        .unwrap_or_default();

    if sym_mid.is_empty() {
        return RegimeMomentumResponse {
            heatmap: Heatmap {
                coins: vec![],
                timeframes: q.timeframes.clone(),
                matrix: vec![],
            },
            transition_matrix: [[0.0; 3]; 3],
            velocity_series: vec![],
            cross_asset_divergence: DivergenceSnapshot {
                leaders: vec![],
                laggards: vec![],
                score: 0.0,
            },
        };
    }

    // ===== Base TF (1h) for velocity series & transition matrix =====
    let tf_base = Tf::H1;
    #[derive(Clone)]
    struct ZS {
        sym: String,
        ts: Vec<i64>,
        z: Vec<f64>,
    }
    let mut zsers: Vec<ZS> = Vec::new();

    for (sym, mid) in &sym_mid {
        if let Some(h) = by_mid.get(mid) {
            let s = resample_from_hourly(h, tf_base.period_secs());
            if s.len() < q.window + q.velocity_window + 6 {
                continue;
            }
            let ts = s.iter().map(|r| r.ts).collect::<Vec<_>>();
            let close = s.iter().map(|r| r.close).collect::<Vec<_>>();
            let z = zscore_series(&close, q.window);
            let start = z.iter().position(|v| v.is_finite()).unwrap_or(z.len());
            if start >= z.len() {
                continue;
            }
            zsers.push(ZS {
                sym: sym.clone(),
                ts: ts[start..].to_vec(),
                z: z[start..].iter().map(|&v| finite(v)).collect(),
            });
        }
    }

    if zsers.is_empty() {
        return RegimeMomentumResponse {
            heatmap: Heatmap {
                coins: vec![],
                timeframes: q.timeframes.clone(),
                matrix: vec![],
            },
            transition_matrix: [[0.0; 3]; 3],
            velocity_series: vec![],
            cross_asset_divergence: DivergenceSnapshot {
                leaders: vec![],
                laggards: vec![],
                score: 0.0,
            },
        };
    }

    // Trim to common length
    let min_len = zsers.iter().map(|s| s.z.len()).min().unwrap();
    for s in &mut zsers {
        if s.z.len() > min_len {
            let off = s.z.len() - min_len;
            s.z.drain(0..off);
            s.ts.drain(0..off);
        }
    }

    // ---- Velocity (rate) & Acceleration on base TF ----
    // velocity(t) = (z[t] - z[t-W]) / W  ; acceleration = Δvelocity
    let w = q.velocity_window.max(1);
    let mut velocity_series: Vec<VelSeries> = Vec::with_capacity(zsers.len());
    let mut last_velocity_by_symbol: Vec<(String, f64)> = Vec::with_capacity(zsers.len());

    for s in &zsers {
        if s.z.len() <= w {
            continue;
        }
        let mut series = Vec::with_capacity(s.z.len() - w);
        for t in w..s.z.len() {
            let vel = finite((s.z[t] - s.z[t - w]) / (w as f64));
            series.push(VelPoint {
                timestamp: s.ts[t],
                velocity: vel,
                acceleration: None,
            });
        }
        for t in 1..series.len() {
            let acc = finite(series[t].velocity - series[t - 1].velocity);
            series[t].acceleration = Some(acc);
        }

        let last_vel = series.last().map(|p| p.velocity).unwrap_or(0.0);
        last_velocity_by_symbol.push((s.sym.clone(), last_vel));
        velocity_series.push(VelSeries {
            symbol: s.sym.clone(),
            series,
        });
    }

    // ---- Regime Transition Matrix on base TF using cross-sectional mean z ----
    let mut mean_z = vec![0.0; min_len];
    for t in 0..min_len {
        let mut sum = 0.0;
        let mut cnt = 0.0;
        for s in &zsers {
            let v = s.z[t];
            if v.is_finite() {
                sum += v;
                cnt += 1.0;
            }
        }
        mean_z[t] = if cnt > 0.0 { sum / cnt } else { 0.0 };
    }
    let mut counts = [[0usize; 3]; 3];
    for t in 1..min_len {
        let a = regime_state_from_z(mean_z[t - 1]);
        let b = regime_state_from_z(mean_z[t]);
        counts[a][b] += 1;
    }
    let mut transition_matrix = [[0.0; 3]; 3];
    for i in 0..3 {
        let row_sum: usize = counts[i].iter().sum();
        if row_sum > 0 {
            for j in 0..3 {
                transition_matrix[i][j] = counts[i][j] as f64 / row_sum as f64;
            }
        }
    }

    // ===== Heatmap over requested TFs: last Δz over scaled window =====
    let mut heat_matrix: Vec<Vec<f64>> = Vec::new();
    for (sym, mid) in &sym_mid {
        let mut row = Vec::with_capacity(q.timeframes.len());
        let Some(h) = by_mid.get(mid) else {
            for _ in &q.timeframes {
                row.push(0.0);
            }
            heat_matrix.push(row);
            continue;
        };

        for tf_str in &q.timeframes {
            let (tf, mult) = parse_tf_and_mult(tf_str);
            let s = resample_from_hourly(h, tf.period_secs());
            let need = q.window + (q.velocity_window * mult) + 6;
            if s.len() < need {
                row.push(0.0);
                continue;
            }

            let close = s.iter().map(|r| r.close).collect::<Vec<_>>();
            let z = zscore_series(&close, q.window);
            let start = z.iter().position(|v| v.is_finite()).unwrap_or(z.len());

            let win = q.velocity_window * mult;
            if start + win >= z.len() {
                row.push(0.0);
                continue;
            }

            let last = z.len() - 1;
            let dz = z[last] - z[last - win];
            row.push(finite(dz));
        }
        heat_matrix.push(row);
    }

    // ===== Cross-asset divergence from last base-TF velocity =====
    last_velocity_by_symbol.sort_by(|a, b| b.1.total_cmp(&a.1));
    let k = ((last_velocity_by_symbol.len() as f64 * 0.2).round() as usize).max(1);
    let leaders = last_velocity_by_symbol
        .iter()
        .take(k)
        .map(|x| x.0.clone())
        .collect::<Vec<_>>();
    let laggards = last_velocity_by_symbol
        .iter()
        .rev()
        .take(k)
        .map(|x| x.0.clone())
        .collect::<Vec<_>>();

    let mean_top = leaders
        .iter()
        .map(|s| {
            last_velocity_by_symbol
                .iter()
                .find(|x| &x.0 == s)
                .unwrap()
                .1
        })
        .sum::<f64>()
        / (k as f64);
    let mean_bot = laggards
        .iter()
        .map(|s| {
            last_velocity_by_symbol
                .iter()
                .find(|x| &x.0 == s)
                .unwrap()
                .1
        })
        .sum::<f64>()
        / (k as f64);

    let mu = last_velocity_by_symbol.iter().map(|x| x.1).sum::<f64>()
        / (last_velocity_by_symbol.len() as f64).max(1.0);
    let var = last_velocity_by_symbol
        .iter()
        .map(|x| {
            let d = x.1 - mu;
            d * d
        })
        .sum::<f64>()
        / (last_velocity_by_symbol.len() as f64).max(1.0);
    let sigma = var.sqrt();
    let div_score = if sigma > 0.0 {
        (mean_top - mean_bot) / sigma
    } else {
        0.0
    };

    RegimeMomentumResponse {
        heatmap: Heatmap {
            coins: sym_mid.iter().map(|(s, _)| s.clone()).collect(),
            timeframes: q.timeframes.clone(),
            matrix: heat_matrix,
        },
        transition_matrix,
        velocity_series,
        cross_asset_divergence: DivergenceSnapshot {
            leaders,
            laggards,
            score: finite(div_score),
        },
    }
}
