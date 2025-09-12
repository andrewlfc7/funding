use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;
use std::sync::OnceLock;

// task-pool infra
use crate::infra::task_pools::{EndpointPool, threads_from_env};

use super::{
    fetch_multi_hourly_ohlcv, log_returns, parse_period_days, resample_from_hourly,
    top_markets_by_usd_volume_live, Tf,
};

// ============== Helpers & Config ==============

fn default_market_type() -> String { "spot".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
fn default_topn() -> i64 { 20 }
fn default_vol_win() -> usize { 24 }    // 1 day on 1h
fn default_vov_win() -> usize { 96 }    // 4 days on 1h
fn default_moment_win() -> usize { 48 } // 2 days on 1h
fn default_cov_win() -> usize { 24 }
fn default_bin_step() -> f64 { 5.0 }    // 5% bins

#[inline]
fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }

#[inline]
fn ms(ts_sec: i64) -> i64 { ts_sec.saturating_mul(1000) }

fn normal_pdf(x: f64, mu: f64, sd: f64) -> f64 {
    if !(sd > 0.0) { return 0.0; }
    let z = (x - mu) / sd;
    let inv_sqrt_2pi = 1.0 / (2.0 * PI).sqrt();
    inv_sqrt_2pi / sd * (-0.5 * z * z).exp()
}

fn sample_mean(xs: &[f64]) -> f64 {
    if xs.is_empty() { return 0.0; }
    finite(xs.iter().copied().filter(|v| v.is_finite()).sum::<f64>() / (xs.len() as f64))
}

fn sample_std(xs: &[f64]) -> f64 {
    let n = xs.len();
    if n < 2 { return 0.0; }
    let mu = sample_mean(xs);
    let var = xs.iter().map(|&v| { let d = finite(v) - mu; d * d }).sum::<f64>() / (n as f64);
    finite(var.sqrt())
}

fn sample_skewness(xs: &[f64]) -> f64 {
    let n = xs.len();
    if n < 3 { return 0.0; }
    let mu = sample_mean(xs);
    let mut m2 = 0.0;
    let mut m3 = 0.0;
    for &v in xs {
        let d = finite(v) - mu;
        m2 += d * d;
        m3 += d * d * d;
    }
    if m2 == 0.0 { return 0.0; }
    let n_f = n as f64;
    finite(n_f / (n_f - 1.0) / (n_f - 2.0) * (m3 / (m2 / n_f).sqrt().powi(3)))
}

fn sample_excess_kurtosis(xs: &[f64]) -> f64 {
    let n = xs.len();
    if n < 4 { return 0.0; }
    let mu = sample_mean(xs);
    let mut m2 = 0.0;
    let mut m4 = 0.0;
    for &v in xs {
        let d = finite(v) - mu;
        m2 += d * d;
        m4 += d.powi(4);
    }
    if m2 == 0.0 { return 0.0; }
    let n_f = n as f64;
    let g2 = (n_f * (n_f + 1.0) * m4 / (m2 * m2) - 3.0 * (n_f - 1.0))
        * ((n_f - 1.0) / ((n_f - 2.0) * (n_f - 3.0)));
    finite(g2)
}

fn covariance(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len().min(ys.len());
    if n < 2 { return 0.0; }
    let xmu = sample_mean(&xs[..n]);
    let ymu = sample_mean(&ys[..n]);
    let mut acc = 0.0;
    for i in 0..n {
        acc += (finite(xs[i]) - xmu) * (finite(ys[i]) - ymu);
    }
    finite(acc / (n as f64))
}

fn rolling_std(xs: &[f64], win: usize) -> Vec<f64> {
    let n = xs.len();
    if win == 0 || n < win { return vec![]; }
    let mut out = Vec::with_capacity(n - win + 1);
    for i in (win - 1)..n {
        let w = &xs[i + 1 - win ..= i];
        out.push(sample_std(w));
    }
    out
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() { return 0.0; }
    let p = p.clamp(0.0, 1.0);
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    finite(sorted[idx])
}

fn linreg_beta_r2(x: &[f64], y: &[f64]) -> (f64, f64) {
    let n = x.len().min(y.len());
    if n < 2 { return (0.0, 0.0) };
    let x = &x[..n];
    let y = &y[..n];
    let xmu = sample_mean(x);
    let ymu = sample_mean(y);
    let mut num = 0.0;
    let mut den = 0.0;
    let mut sst = 0.0;
    let mut sse = 0.0;
    for i in 0..n {
        let xd = finite(x[i]) - xmu;
        let yd = finite(y[i]) - ymu;
        num += xd * yd;
        den += xd * xd;
    }
    let beta = if den > 0.0 { num / den } else { 0.0 };
    for i in 0..n {
        let y_hat = ymu + beta * (finite(x[i]) - xmu);
        sst += (finite(y[i]) - ymu).powi(2);
        sse += (finite(y[i]) - y_hat).powi(2);
    }
    let r2 = if sst > 0.0 { 1.0 - sse / sst } else { 0.0 };
    (finite(beta), finite(r2.clamp(0.0, 1.0)))
}

// ============== Request & Response DTOs ==============

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolatilityDynamicsRequest {
    pub exchange: String,
    #[serde(default = "default_market_type")]
    pub market_type: String,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    pub period: String,
    #[serde(default = "default_topn")]
    pub top_n: i64,
    #[serde(default)]
    pub index_coin: Option<String>,

    // windows
    #[serde(default = "default_vol_win")]
    pub vol_window: usize,
    #[serde(default = "default_vov_win")]
    pub vov_window: usize,
    #[serde(default = "default_moment_win")]
    pub moment_window: usize,
    #[serde(default = "default_cov_win")]
    pub cov_window: usize,

    // histogram options
    #[serde(default)]
    pub histogram_coin: Option<String>,
    #[serde(default = "default_bin_step")]
    pub bin_step_pct: f64,
}

#[derive(Debug, Serialize)]
pub struct Point { pub timestamp: i64, pub value: f64 }

#[derive(Debug, Serialize)]
pub struct Series1D { pub symbol: String, pub data: Vec<Point> }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolatilityDynamicsResponse {
    pub vov_time_series: Vec<Series1D>,
    pub skewness_time_series: Vec<Series1D>,
    pub skew_kurtosis_scatter: Vec<ScatterSK>,
    pub vol_distribution: VolDistribution,
    pub distribution_stats: Vec<DistStat>,
    pub covariance_time_series: Vec<Series1D>,
    pub regime_classification: Vec<RegimeClassRow>,
    pub instability_rankings: Vec<InstabilityRow>,
    pub flow_vol_beta: Vec<FlowVolBeta>,
    pub vol_stats_summary: Vec<VolStatsRow>,
}

#[derive(Debug, Serialize)]
pub struct ScatterSK { pub symbol: String, pub skewness: f64, pub kurtosis: f64 }

#[derive(Debug, Serialize)]
pub struct VolDistribution { pub bins: Vec<f64>, pub frequencies: Vec<usize>, pub normal_curve: Vec<f64> }

#[derive(Debug, Serialize)]
pub struct DistStat { pub label: String, pub value: f64, pub line: bool, pub color: Option<String> }

#[derive(Debug, Serialize)]
pub struct RegimeClassRow {
    pub symbol: String,
    pub regime: String,    // 'normal' | 'stressed' | 'euphoric' | 'compressed' | 'unstable'
    pub volatility: f64,   // %
    pub skewness: f64,
    pub kurtosis: f64,
}

#[derive(Debug, Serialize)]
pub struct InstabilityRow {
    pub symbol: String,
    pub vov: f64,
    pub skewness: f64,
    pub status: String,    // 'critical' | 'warning' | 'normal'
    pub status_label: String,
}

#[derive(Debug, Serialize)]
pub struct FlowVolBeta { pub symbol: String, pub beta: f64, pub r_squared: f64 }

#[derive(Debug, Serialize)]
pub struct VolStatsRow {
    pub symbol: String,
    pub current_vol: f64,
    pub avg_vol: f64,
    pub percentile: f64,
    pub skewness: f64,
    pub kurtosis: f64,
    pub vov: f64,
}

// ================= Task-pool wiring =================

struct Job { pool: PgPool, q: VolatilityDynamicsRequest }

static VOL_DYN_POOL: OnceLock<EndpointPool<Job, VolatilityDynamicsResponse>> = OnceLock::new();

fn vol_dyn_pool() -> &'static EndpointPool<Job, VolatilityDynamicsResponse> {
    VOL_DYN_POOL.get_or_init(|| {
        let n = threads_from_env("VOL_DYN_THREADS", 2);
        EndpointPool::start("volatility-dynamics", n, |job: Job| async move {
            compute_volatility_dynamics(job.pool, job.q).await
        })
    })
}

// ============== Thin handler ==============

pub async fn get_volatility_dynamics(
    State(db): State<PgPool>,
    Query(q): Query<VolatilityDynamicsRequest>,
) -> Json<VolatilityDynamicsResponse> {
    let res = vol_dyn_pool().run(Job { pool: db.clone(), q }).await;
    Json(res)
}

// ============== Heavy compute ==============

async fn compute_volatility_dynamics(
    pool: PgPool,
    q: VolatilityDynamicsRequest,
) -> VolatilityDynamicsResponse {
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    let days = parse_period_days(&q.period);
    let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();

    // universe
    let mut symset: BTreeSet<String> = BTreeSet::new();
    if let Ok(top) = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.market_type, days as i32, q.top_n).await {
        for (s, _, _) in top { symset.insert(s.to_uppercase()); }
    }
    if let Some(idx) = q.index_coin.as_ref() { symset.insert(idx.to_uppercase()); }
    let syms: Vec<String> = symset.into_iter().collect();
    if syms.is_empty() {
        return empty_response();
    }

    // resolve + fetch
    let mut mids = Vec::new();
    let mut sym_mid = Vec::new();
    for s in &syms {
        if let Ok(mid) = super::resolve_market_id_with_data(&pool, &q.exchange, s, &q.market_type, since_unix).await {
            mids.push(mid);
            sym_mid.push((s.clone(), mid));
        }
    }
    if sym_mid.is_empty() {
        return empty_response();
    }
    let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix).await.unwrap_or_default();

    // prepare per-asset series (ts, returns, usd)
    #[derive(Clone)]
    struct ASer {
        sym: String,
        ts: Vec<i64>,       // seconds
        close: Vec<f64>,
        usd: Vec<f64>,
        lr: Vec<f64>,       // log returns
        lr_ts: Vec<i64>,    // ts aligned to lr (ts[1..])
    }
    let mut aset: Vec<ASer> = Vec::new();
    for (sym, mid) in &sym_mid {
        if let Some(h) = by_mid.get(mid) {
            let s = resample_from_hourly(h, tf.period_secs());
            if s.len() < 10 { continue; }
            let ts: Vec<i64> = s.iter().map(|r| r.ts).collect();
            let close: Vec<f64> = s.iter().map(|r| finite(r.close)).collect();
            let base: Vec<f64>  = s.iter().map(|r| finite(r.volume)).collect();
            let usd: Vec<f64>   = close.iter().zip(base.iter()).map(|(p, v)| finite(p * v)).collect();

            let lr = log_returns(&close);
            if lr.is_empty() { continue; }
            let lr_ts = ts.iter().copied().skip(1).collect::<Vec<_>>();

            aset.push(ASer { sym: sym.clone(), ts, close, usd, lr, lr_ts });
        }
    }
    if aset.is_empty() {
        return empty_response();
    }

    // Determine index series (defaults to first if not found)
    let idx_sym = q.index_coin.as_ref().map(|s| s.to_uppercase());
    let idx_pos = idx_sym
        .as_ref()
        .and_then(|s| aset.iter().position(|a| &a.sym == s))
        .unwrap_or(0);
    let idx = aset[idx_pos].clone();

    // ---------------- VoV (rolling std of rolling std) & Skewness ----------------
    let mut vov_ts_all: Vec<Series1D> = Vec::new();
    let mut skew_ts_all: Vec<Series1D> = Vec::new();

    for s in &aset {
        // rolling vol of returns
        let vol = rolling_std(&s.lr, q.vol_window);              // length V
        let vol_ts = if s.lr_ts.len() >= q.vol_window {
            s.lr_ts.iter().copied().skip(q.vol_window - 1).collect::<Vec<_>>() // V timestamps
        } else { vec![] };

        // vov over vol
        let vov = rolling_std(&vol, q.vov_window);               // length W
        let vov_ts = if vol_ts.len() >= q.vov_window {
            vol_ts.iter().copied().skip(q.vov_window - 1).collect::<Vec<_>>() // W timestamps
        } else { vec![] };

        // clamp by min length
        let w = vov.len().min(vov_ts.len());
        let mut vov_points = Vec::with_capacity(w);
        for i in 0..w {
            vov_points.push(Point { timestamp: ms(vov_ts[i]), value: finite(vov[i]) });
        }
        vov_ts_all.push(Series1D { symbol: s.sym.clone(), data: vov_points });

        // skewness over returns
        let mut skew_points = Vec::new();
        if s.lr.len() >= q.moment_window && s.lr_ts.len() >= q.moment_window {
            let end = s.lr.len().min(s.lr_ts.len());
            for i in (q.moment_window - 1)..end {
                let win = &s.lr[i + 1 - q.moment_window ..= i];
                let sk = finite(sample_skewness(win));
                skew_points.push(Point { timestamp: ms(s.lr_ts[i]), value: sk });
            }
        }
        skew_ts_all.push(Series1D { symbol: s.sym.clone(), data: skew_points });
    }

    // ---------------- Scatter (current skew/kurt) ----------------
    let mut scatter: Vec<ScatterSK> = Vec::new();
    for s in &aset {
        let (sk, ku) = if s.lr.len() >= q.moment_window {
            let win = &s.lr[s.lr.len() - q.moment_window ..];
            (finite(sample_skewness(win)), finite(sample_excess_kurtosis(win)))
        } else { (0.0, 0.0) };
        scatter.push(ScatterSK { symbol: s.sym.clone(), skewness: sk, kurtosis: ku });
    }

    // ---------------- Vol Distribution (for selected coin or index) ----------------
    let dist_sym = q.histogram_coin.as_ref()
        .and_then(|x| aset.iter().find(|a| &a.sym == x))
        .cloned()
        .unwrap_or_else(|| idx.clone());

    // rolling vol (%)
    let vol_pct = {
        let v = rolling_std(&dist_sym.lr, q.vol_window)
            .into_iter().map(|x| finite(x * 100.0)).collect::<Vec<_>>();
        v
    };
    // bins
    let mut bins = Vec::new();
    let step = q.bin_step_pct.max(1.0);
    let max_val = (vol_pct.iter().copied().fold(0.0_f64, f64::max) / step).ceil() * step;
    let mut b = 0.0_f64;
    while b <= max_val + step {
        bins.push(b);
        b += step;
    }

    // frequencies
    let mut freqs = vec![0usize; bins.len().saturating_sub(1)];
    for &v in &vol_pct {
        for i in 0..freqs.len() {
            let left = bins[i];
            let right = bins[i + 1];
            let last = i == freqs.len() - 1;
            if (v >= left && v < right) || (last && v == right) {
                freqs[i] += 1;
                break;
            }
        }
    }

    // normal curve based on sample mean/std of vol_pct
    let mu = sample_mean(&vol_pct);
    let sd = sample_std(&vol_pct);
    let total = freqs.iter().sum::<usize>().max(1) as f64;
    let mut curve = Vec::with_capacity(freqs.len());
    for i in 0..freqs.len() {
        let mid = 0.5 * (bins[i] + bins[i + 1]);
        let pdf = normal_pdf(mid, mu, sd);
        curve.push(finite(pdf * total * step / 10.0)); // heuristic scaling
    }

    // Dist stats
    let mut sorted = vol_pct.clone();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let stats = vec![
        DistStat { label: "Mean".into(),  value: finite(mu), line: true,  color: Some("#60a5fa".into()) },
        DistStat { label: "−1σ".into(),   value: finite(mu - sd), line: true,  color: Some("#f59e0b".into()) },
        DistStat { label: "+1σ".into(),   value: finite(mu + sd), line: true,  color: Some("#f59e0b".into()) },
        DistStat { label: "95th %ile".into(), value: finite(percentile(&sorted, 0.95)), line: true, color: Some("#ef4444".into()) },
    ];

    // ---------------- Rolling Covariance (vs index) ----------------
    let idx_map: BTreeMap<i64, f64> = idx.lr_ts.iter().copied().zip(idx.lr.iter().copied().map(finite)).collect();
    let mut cov_series_all: Vec<Series1D> = Vec::new();

    for s in &aset {
        if s.sym == idx.sym { continue; }
        let mut pairs: Vec<(i64, f64, f64)> = Vec::new();
        for (t, r) in s.lr_ts.iter().copied().zip(s.lr.iter().copied().map(finite)) {
            if let Some(&ri) = idx_map.get(&t) {
                pairs.push((t, ri, r));
            }
        }
        // rolling covariance over cov_window
        let n = pairs.len();
        let mut pts = Vec::new();
        if n >= q.cov_window {
            let mut buf_x = Vec::with_capacity(q.cov_window);
            let mut buf_y = Vec::with_capacity(q.cov_window);
            for i in 0..n {
                let (_, xi, yi) = pairs[i];
                buf_x.push(finite(xi));
                buf_y.push(finite(yi));
                if buf_x.len() > q.cov_window {
                    buf_x.remove(0);
                    buf_y.remove(0);
                }
                if buf_x.len() == q.cov_window {
                    let t = pairs[i].0;
                    let c = finite(covariance(&buf_x, &buf_y));
                    pts.push(Point { timestamp: ms(t), value: c });
                }
            }
        }
        cov_series_all.push(Series1D { symbol: s.sym.clone(), data: pts });
    }

    // ---------------- Regime classification & instability ----------------
    let mut regime_rows: Vec<RegimeClassRow> = Vec::new();
    let mut inst_rows: Vec<InstabilityRow> = Vec::new();
    let mut flow_beta_rows: Vec<FlowVolBeta> = Vec::new();
    let mut summary_rows: Vec<VolStatsRow> = Vec::new();

    for s in &aset {
        // rolling vol (%)
        let vol_pct_s = rolling_std(&s.lr, q.vol_window).into_iter().map(|x| finite(x * 100.0)).collect::<Vec<_>>();
        let cur_vol = vol_pct_s.last().copied().unwrap_or(0.0);
        let avg_vol = if !vol_pct_s.is_empty() { sample_mean(&vol_pct_s) } else { 0.0 };
        let sd_vol  = if vol_pct_s.len() > 1 { sample_std(&vol_pct_s) } else { 0.0 };
        let mut sorted = vol_pct_s.clone();
        sorted.sort_by(|a, b| a.total_cmp(b));
        let pct = if !sorted.is_empty() {
            let pos = match sorted.binary_search_by(|v| v.total_cmp(&cur_vol)) {
                Ok(i) => i,
                Err(i) => i,
            };
            finite((pos as f64) / (sorted.len() as f64) * 100.0)
        } else { 0.0 };

        // skew/kurt
        let (sk, ku) = if s.lr.len() >= q.moment_window {
            let w = &s.lr[s.lr.len() - q.moment_window ..];
            (finite(sample_skewness(w)), finite(sample_excess_kurtosis(w)))
        } else { (0.0, 0.0) };

        // VoV latest
        let v = rolling_std(&s.lr, q.vol_window);
        let vov_vec = rolling_std(&v, q.vov_window);
        let latest_vov = vov_vec.iter().rev().find(|x| x.is_finite()).copied().unwrap_or(0.0);

        // regime rule-of-thumb
        let regime = if cur_vol > avg_vol + 2.0 * sd_vol {
            "stressed"
        } else if cur_vol < (avg_vol - 1.0 * sd_vol).max(0.0) {
            "compressed"
        } else if sk > 1.0 && ku > 1.0 {
            "euphoric"
        } else if sk.abs() > 1.5 {
            "unstable"
        } else {
            "normal"
        }.to_string();

        regime_rows.push(RegimeClassRow {
            symbol: s.sym.clone(),
            regime,
            volatility: finite(cur_vol),
            skewness: sk,
            kurtosis: ku,
        });

        // instability grading from VoV + skew
        let status = if latest_vov > 0.02 && sk.abs() > 1.0 { "critical" }
                     else if latest_vov > 0.01 || sk.abs() > 0.7 { "warning" }
                     else { "normal" };
        let status_label = match status {
            "critical" => "Critical",
            "warning"  => "Warning",
            _          => "Stable",
        }.to_string();

        inst_rows.push(InstabilityRow {
            symbol: s.sym.clone(),
            vov: finite(latest_vov),
            skewness: sk,
            status: status.to_string(),
            status_label,
        });

        // flow-vol beta (usd vs vol%)
        let (beta, r2) = {
            let n = vol_pct_s.len().min(s.usd.len());
            if n >= 10 {
                let xs = &s.usd[s.usd.len() - n ..];
                let ys = &vol_pct_s[vol_pct_s.len() - n ..];
                linreg_beta_r2(xs, ys)
            } else { (0.0, 0.0) }
        };
        flow_beta_rows.push(FlowVolBeta { symbol: s.sym.clone(), beta: finite(beta), r_squared: finite(r2) });

        summary_rows.push(VolStatsRow {
            symbol: s.sym.clone(),
            current_vol: finite(cur_vol),
            avg_vol: finite(avg_vol),
            percentile: finite(pct),
            skewness: sk,
            kurtosis: ku,
            vov: finite(latest_vov),
        });
    }

    VolatilityDynamicsResponse {
        vov_time_series: vov_ts_all,
        skewness_time_series: skew_ts_all,
        skew_kurtosis_scatter: scatter,
        vol_distribution: VolDistribution { bins, frequencies: freqs, normal_curve: curve },
        distribution_stats: stats,
        covariance_time_series: cov_series_all,
        regime_classification: regime_rows,
        instability_rankings: inst_rows,
        flow_vol_beta: flow_beta_rows,
        vol_stats_summary: summary_rows,
    }
}

fn empty_response() -> VolatilityDynamicsResponse {
    VolatilityDynamicsResponse {
        vov_time_series: vec![],
        skewness_time_series: vec![],
        skew_kurtosis_scatter: vec![],
        vol_distribution: VolDistribution { bins: vec![], frequencies: vec![], normal_curve: vec![] },
        distribution_stats: vec![],
        covariance_time_series: vec![],
        regime_classification: vec![],
        instability_rankings: vec![],
        flow_vol_beta: vec![],
        vol_stats_summary: vec![],
    }
}
