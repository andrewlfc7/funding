use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::OnceLock;

use crate::infra::task_pools::{EndpointPool, threads_from_env};

use super::{
    fetch_multi_hourly_ohlcv, histogram_counts, log_returns, parse_period_days,
    resample_from_hourly, rolling_mean_std, top_markets_by_usd_volume_live, zscore_series, Tf,
};

fn default_market_type() -> String { "spot".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
fn default_xsec() -> String { "24h".to_string() }
#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }

#[derive(Clone, Debug, Deserialize)]
pub struct LeadersLaggardsRequest {
    pub exchange: String,
    pub period: String,                       // data lookback, e.g. "30d"
    #[serde(default = "default_market_type")]
    pub marketType: String,
    #[serde(default)]
    pub topN: Option<i64>,                    // universe size (default 30)
    #[serde(default = "default_timeframe")]
    pub timeframe: String,                    // "1h"|"4h"|"1d"
    #[serde(default = "default_xsec")]
    pub xsec: String,                         // "1h"|"24h"|"7d" for cross-section horizon
}

#[derive(Debug, Serialize)]
pub struct LeadersLaggardsResponse {
    pub leaders: Vec<XRow>,
    pub laggards: Vec<XRow>,
    pub volumeSpikes: Vec<VolSpike>,
    pub decorrelated: Vec<DecorRow>,
    pub leadLagMatrix: LeadLagMatrix,
}

#[derive(Debug, Serialize, Clone)]
pub struct XRow {
    pub symbol: String,
    pub zscore: f64,
    pub returns: f64,
    pub volume: f64,
    pub rank: usize,
}

#[derive(Debug, Serialize)]
pub struct VolSpike {
    pub symbol: String,
    pub volumeZScore: f64,  // z of EWMA(USD vol)
    pub priceChange: f64,   // last 1-bar return
}

#[derive(Debug, Serialize)]
pub struct DecorRow {
    pub symbol: String,
    pub correlationWithMarket: f64,
    pub avgCorrelation: f64,
}

#[derive(Debug, Serialize)]
pub struct LeadLagMatrix {
    pub coins: Vec<String>,
    pub lags: Vec<i32>,                 // hours, e.g. [-6,-3,0,3,6]
    pub matrix: Vec<Vec<Vec<f64>>>,     // [lag][i][j]
}

fn steps_for_xsec(tf: Tf, xsec: &str) -> usize {
    match xsec {
        "1h" => 1.max(1),
        "7d" => tf.steps_per_day() * 7,
        _    => tf.steps_per_day(), // "24h" default
    }
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n == 0 { return 0.0; }
    let (mut sx,mut sy,mut sxx,mut syy,mut sxy)=(0.0,0.0,0.0,0.0,0.0);
    let mut m=0usize;
    for i in 0..n {
        let (a,b)=(x[i],y[i]);
        if a.is_finite() && b.is_finite() { sx+=a; sy+=b; sxx+=a*a; syy+=b*b; sxy+=a*b; m+=1; }
    }
    if m<=1 { return 0.0; }
    let mf = m as f64;
    let cov = sxy - sx*sy/mf;
    let vx  = sxx - sx*sx/mf;
    let vy  = syy - sy*sy/mf;
    if vx<=0.0 || vy<=0.0 { 0.0 } else { finite(cov/(vx.sqrt()*vy.sqrt())) }
}

// corr( x_t , y_{t+lag} ), positive lag = y leads
fn shift_corr(x: &[f64], y: &[f64], lag: isize) -> f64 {
    if x.is_empty() || y.is_empty() { return 0.0; }
    let (mut a, mut b) = (Vec::new(), Vec::new());
    if lag > 0 {
        let l = lag as usize;
        let n = x.len().min(y.len().saturating_sub(l));
        for i in 0..n { a.push(x[i]); b.push(y[i+l]); }
    } else if lag < 0 {
        let l = (-lag) as usize;
        let n = x.len().saturating_sub(l).min(y.len());
        for i in 0..n { a.push(x[i+l]); b.push(y[i]); }
    } else {
        let n = x.len().min(y.len());
        for i in 0..n { a.push(x[i]); b.push(y[i]); }
    }
    pearson(&a, &b)
}

// ---------- Task pool plumbing ----------

#[derive(Clone)]
struct Job { pool: PgPool, q: LeadersLaggardsRequest }

static LL_POOL: OnceLock<EndpointPool<Job, LeadersLaggardsResponse>> = OnceLock::new();

fn pool() -> &'static EndpointPool<Job, LeadersLaggardsResponse> {
    LL_POOL.get_or_init(|| {
        let n = threads_from_env("LEADERS_LAGGARDS_THREADS", 2);
        EndpointPool::start("leaders-laggards", n, |job: Job| async move {
            compute_leaders_laggards(job.pool, job.q).await
        })
    })
}

// ---------- Handler (dispatches to pool) ----------

pub async fn get_leaders_laggards(
    State(pool_state): State<PgPool>,
    Query(q): Query<LeadersLaggardsRequest>,
) -> Json<LeadersLaggardsResponse> {
    let res = pool().run(Job { pool: pool_state.clone(), q }).await;
    Json(res)
}

// ---------- Heavy computation (runs inside pool runtime) ----------

async fn compute_leaders_laggards(
    pool: PgPool,
    q: LeadersLaggardsRequest,
) -> LeadersLaggardsResponse {
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    let days = parse_period_days(&q.period);
    let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();
    let topn = q.topN.unwrap_or(30);

    // Universe by USD notional
    let top = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.marketType, days as i32, topn)
        .await
        .unwrap_or_default();
    if top.is_empty() {
        return LeadersLaggardsResponse {
            leaders: vec![], laggards: vec![], volumeSpikes: vec![],
            decorrelated: vec![], leadLagMatrix: LeadLagMatrix { coins: vec![], lags: vec![], matrix: vec![] },
        };
    }

    let mids: Vec<i32> = top.iter().map(|(_, mid, _)| *mid).collect();
    let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix).await.unwrap_or_default();

    // Collect per-coin metrics
    struct PerCoin {
        sym: String,
        ts: Vec<i64>,
        close: Vec<f64>,
        usd: Vec<f64>,
        lr: Vec<f64>,
        last_z: f64,
        last_ret_h: f64,
        last_usd: f64,
        volz_last: f64,
    }

    let mut coins: Vec<PerCoin> = Vec::new();
    let horizon = steps_for_xsec(tf, &q.xsec);

    for (sym, mid, _) in top {
        let Some(hourly) = by_mid.get(&mid) else { continue; };
        let s = resample_from_hourly(hourly, tf.period_secs());
        if s.len() < (horizon + 30) { continue; }

        let ts: Vec<i64> = s.iter().map(|r| r.ts).collect();
        let close: Vec<f64> = s.iter().map(|r| r.close).collect();
        let base: Vec<f64> = s.iter().map(|r| r.volume).collect();
        let usd: Vec<f64> = close.iter().zip(base.iter()).map(|(p,&v)| finite(p*v)).collect();

        // price zscore
        let win = (tf.steps_per_day()*5).max(24).min(close.len().saturating_sub(1));
        let z = zscore_series(&close, win);
        let start = z.iter().position(|v| v.is_finite()).unwrap_or(z.len());
        if start >= z.len() { continue; }

        // log returns
        let lr = log_returns(&close);

        // EWMA(USD) z for spikes
        let vol_ewma = super::ewma_span(&usd, win.max(24));
        let volz = zscore_series(&vol_ewma, win);

        // horizon return
        let n = close.len();
        let last_idx = n - 1;
        let ret_h = if last_idx >= horizon {
            let p0 = close[last_idx - horizon];
            if p0 > 0.0 { (close[last_idx] / p0) - 1.0 } else { 0.0 }
        } else { 0.0 };

        coins.push(PerCoin{
            sym,
            ts: ts[start..].to_vec(),
            close: close[start..].to_vec(),
            usd: usd[start..].to_vec(),
            lr: lr[start..].to_vec(),
            last_z: finite(*z[start..].last().unwrap()),
            last_ret_h: finite(ret_h),
            last_usd: finite(*usd.last().unwrap()),
            volz_last: finite(*volz[start..].last().unwrap()),
        });
    }

    if coins.is_empty() {
        return LeadersLaggardsResponse {
            leaders: vec![], laggards: vec![], volumeSpikes: vec![],
            decorrelated: vec![], leadLagMatrix: LeadLagMatrix { coins: vec![], lags: vec![], matrix: vec![] },
        };
    }

    // Leaders/Laggards by last z
    let mut rows: Vec<XRow> = coins.iter().map(|c| XRow {
        symbol: c.sym.clone(),
        zscore: c.last_z,
        returns: c.last_ret_h,
        volume: c.last_usd,
        rank: 0,
    }).collect();

    rows.sort_by(|a,b| b.zscore.total_cmp(&a.zscore));
    for (i, r) in rows.iter_mut().enumerate() { r.rank = i + 1; }

    let leaders = rows.iter().take(10).cloned().collect::<Vec<_>>();
    let laggards = rows.iter().rev().take(10).cloned().collect::<Vec<_>>();

    // Volume spikes
    let mut spikes: Vec<VolSpike> = coins.iter().map(|c| VolSpike {
        symbol: c.sym.clone(),
        volumeZScore: c.volz_last,
        priceChange: c.lr.last().copied().unwrap_or(0.0),
    }).collect();
    spikes.sort_by(|a,b| b.volumeZScore.total_cmp(&a.volumeZScore));
    spikes.truncate(15);

    // Market Index: custom weighted index of BTC, ETH, and SOL.
    // Weights are 50% BTC (overweight), 30% ETH, and 20% SOL (underweight).
    const BTC_W: f64 = 0.50;
    const ETH_W: f64 = 0.30;
    const SOL_W: f64 = 0.20;

    let btc_coin = coins.iter().find(|c| c.sym.starts_with("btc"));
    let eth_coin = coins.iter().find(|c| c.sym.starts_with("eth"));
    let sol_coin = coins.iter().find(|c| c.sym.starts_with("sol"));

    let market: Vec<f64> = if let (Some(btc), Some(eth), Some(sol)) = (btc_coin, eth_coin, sol_coin) {
        let min_len_idx = btc.lr.len().min(eth.lr.len()).min(sol.lr.len());
        if min_len_idx > 0 {
            let btc_lr = &btc.lr[btc.lr.len() - min_len_idx..];
            let eth_lr = &eth.lr[eth.lr.len() - min_len_idx..];
            let sol_lr = &sol.lr[sol.lr.len() - min_len_idx..];

            // Calculate the weighted average return for each period.
            (0..min_len_idx)
                .map(|i| BTC_W * btc_lr[i] + ETH_W * eth_lr[i] + SOL_W * sol_lr[i])
                .collect()
        } else {
            vec![] // Not enough data from one of the index constituents.
        }
    } else {
        // Fallback to equal-weight if custom index coins are not in the universe.
        let min_len_all = coins.iter().map(|c| c.lr.len()).min().unwrap_or(0);
        if min_len_all > 0 {
            let mut market_returns = vec![0.0; min_len_all];
            for c in &coins {
                let lr = &c.lr[c.lr.len() - min_len_all..];
                for i in 0..min_len_all {
                    market_returns[i] += lr[i];
                }
            }
            let k = coins.len() as f64;
            if k > 0.0 {
                market_returns.iter_mut().for_each(|v| *v /= k);
            }
            market_returns
        } else {
            vec![]
        }
    };

    // Decorrelated list
    let mut decor: Vec<DecorRow> = Vec::new();
    for c in &coins {
        // Correlate coin `c` with the calculated market index.
        // This requires aligning the two time series to their common length.
        let common_len_mkt = c.lr.len().min(market.len());
        let corr_mkt = if common_len_mkt > 0 {
            let lr_c_mkt = &c.lr[c.lr.len() - common_len_mkt..];
            let market_aligned = &market[market.len() - common_len_mkt..];
            pearson(lr_c_mkt, market_aligned)
        } else {
            0.0
        };

        // Calculate average correlation of coin `c` with all other coins in the universe.
        // This also uses pairwise alignment for robustness.
        let mut sum_corr = 0.0;
        let mut count = 0;
        for d in &coins {
            if std::ptr::eq(c, d) { continue; } // Skip self-correlation
            let common_len_pair = c.lr.len().min(d.lr.len());
            if common_len_pair > 0 {
                let lr_c_pair = &c.lr[c.lr.len() - common_len_pair..];
                let lr_d_pair = &d.lr[d.lr.len() - common_len_pair..];
                sum_corr += pearson(lr_c_pair, lr_d_pair);
                count += 1;
            }
        }
        let avg_corr = if count > 0 { sum_corr / (count as f64) } else { 0.0 };

        decor.push(DecorRow {
            symbol: c.sym.clone(),
            correlationWithMarket: finite(corr_mkt),
            avgCorrelation: finite(avg_corr),
        });
    }
    decor.sort_by(|a, b| a.correlationWithMarket.total_cmp(&a.correlationWithMarket));
    decor.truncate(15);


    // Lead-lag matrix (limit to 10 coins)
    let mut top_syms: Vec<String> = rows.iter().map(|r| r.symbol.clone()).collect();
    let keep = top_syms.len().min(10);
    top_syms.truncate(keep);

    let mut sub: Vec<&PerCoin> = Vec::new();
    for s in &top_syms {
        if let Some(c) = coins.iter().find(|x| &x.sym == s) { sub.push(c); }
    }
    let lags: Vec<i32> = vec![-6, -3, 0, 3, 6];
    let mut matrix: Vec<Vec<Vec<f64>>> = Vec::new(); // [lag][i][j]
    for &lag in &lags {
        let mut m = vec![vec![0.0; keep]; keep];
        for i in 0..keep {
            for j in 0..keep {
                let a = &sub[i].lr;
                let b = &sub[j].lr;
                m[i][j] = finite(shift_corr(a, b, lag as isize));
            }
        }
        matrix.push(m);
    }

    LeadersLaggardsResponse {
        leaders,
        laggards,
        volumeSpikes: spikes,
        decorrelated: decor,
        leadLagMatrix: LeadLagMatrix { coins: top_syms, lags, matrix },
    }
}