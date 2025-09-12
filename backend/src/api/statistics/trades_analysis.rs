use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::f64::consts::PI;
use std::sync::OnceLock;
use time::OffsetDateTime;

// task-pool infra
use crate::infra::task_pools::{EndpointPool, threads_from_env};

// ----------------- Helpers -----------------

fn default_market_type() -> String { "spot".to_string() }
fn default_period() -> String { "30d".to_string() }
fn default_topn() -> i64 { 20 }
fn default_interval_min() -> i64 { 1 }
fn default_whale_pct() -> f64 { 0.99 }

#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }
#[inline] fn ms(ts_sec: i64) -> i64 { ts_sec.saturating_mul(1000) }
#[inline] fn floor_minute(sec: i64, step_min: i64) -> i64 {
    let step = step_min.max(1) * 60;
    sec - (sec % step)
}

fn parse_period_days(s: &str) -> i64 {
    let s = s.trim().to_ascii_lowercase();
    if let Some(p) = s.strip_suffix("d") { p.parse::<i64>().unwrap_or(30) } else { 30 }
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() { return 0.0; }
    finite(xs.iter().copied().sum::<f64>() / xs.len() as f64)
}

fn stddev(xs: &[f64]) -> f64 {
    if xs.len() < 2 { return 0.0; }
    let mu = mean(xs);
    let var = xs.iter().map(|&v| { let d = v - mu; d*d }).sum::<f64>() / xs.len() as f64;
    finite(var.sqrt())
}

fn percentile(mut xs: Vec<f64>, p: f64) -> f64 {
    if xs.is_empty() { return 0.0; }
    xs.sort_by(|a,b| a.total_cmp(b));
    let p = p.clamp(0.0, 1.0);
    let idx = ((xs.len() as f64 - 1.0) * p).round() as usize;
    finite(xs[idx])
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len());
    if n < 2 { return 0.0; }
    let (x, y) = (&x[..n], &y[..n]);
    let mx = mean(x); let my = mean(y);
    let mut sxy=0.0; let mut sxx=0.0; let mut syy=0.0;
    for i in 0..n {
        let dx = x[i]-mx; let dy = y[i]-my;
        sxy += dx*dy; sxx += dx*dx; syy += dy*dy;
    }
    if sxx==0.0 || syy==0.0 { return 0.0; }
    finite(sxy / (sxx.sqrt()*syy.sqrt()))
}

fn lognormal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    if !(x>0.0) || !(sigma>0.0) { return 0.0; }
    let z = (x.ln() - mu) / sigma;
    (1.0 / (x * sigma * (2.0*PI).sqrt())) * (-0.5 * z * z).exp()
}

// ----------------- DTOs -----------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradesAnalysisRequest {
    pub exchange: String,
    #[serde(default = "default_market_type")]
    pub market_type: String,
    #[serde(default = "default_period")]
    pub period: String,
    #[serde(default = "default_topn")]
    pub top_n: i64,

    #[serde(default)]
    pub coins: Option<Vec<String>>,

    #[serde(default = "default_interval_min")]
    pub interval_min: i64,

    #[serde(default)]
    pub histogram_symbol: Option<String>,

    #[serde(default = "default_whale_pct")]
    pub whale_percentile: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradesAnalysisResponse {
    pub imbalance_scatter: Vec<ScatterRow>,
    pub size_distribution: SizeDistribution,
    pub notional_rankings: Vec<NotionalRow>,
    pub imbalance_time_series: Vec<Series1D>,
    pub velocity_heatmap: Vec<Vec<f64>>,
    pub whale_trades: Vec<WhaleRow>,
    pub correlation_matrix: Vec<Vec<f64>>,
    pub trade_z_scores: Vec<ZRow>,
    pub microstructure_stats: Vec<StatRow>,
}

#[derive(Debug, Serialize)]
pub struct ScatterRow {
    pub symbol: String,
    pub buy_ratio: f64,
    pub volume: f64,
    pub avg_trade_size: f64,
    pub imbalance: f64,
}

#[derive(Debug, Serialize)]
pub struct SizeDistribution {
    pub bins: Vec<f64>,
    pub frequencies: Vec<usize>,
    pub normal_curve: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct NotionalRow {
    pub symbol: String,
    pub avg_size: f64,
    pub interval: String,
}

#[derive(Debug, Serialize)]
pub struct Point { pub timestamp: i64, pub value: f64 }
#[derive(Debug, Serialize)]
pub struct Series1D { pub symbol: String, pub data: Vec<Point> }

#[derive(Debug, Serialize)]
pub struct WhaleRow {
    pub id: String,
    pub symbol: String,
    pub timestamp: i64,
    pub size: f64,
    pub price: f64,
    pub side: String,
}

#[derive(Debug, Serialize)]
pub struct ZRow {
    pub name: String,
    #[serde(rename="type")]
    pub kind: String,
    pub current: f64,
    pub average: f64,
    pub zscore: f64,
}

#[derive(Debug, Serialize)]
pub struct StatRow { pub name: String, pub value: String }

// ----------------- task pool wiring -----------------

struct Job { pool: PgPool, q: TradesAnalysisRequest }

static TRADES_POOL: OnceLock<EndpointPool<Job, TradesAnalysisResponse>> = OnceLock::new();

fn trades_pool() -> &'static EndpointPool<Job, TradesAnalysisResponse> {
    TRADES_POOL.get_or_init(|| {
        let n = threads_from_env("TRADES_ANALYSIS_THREADS", 2);
        EndpointPool::start("trades-analysis", n, |job: Job| async move {
            compute_trades_analysis(job.pool, job.q).await
        })
    })
}

// ----------------- Handler (thin) -----------------

pub async fn get_trades_analysis(
    State(db): State<PgPool>,
    Query(q): Query<TradesAnalysisRequest>,
) -> Json<TradesAnalysisResponse> {
    let res = trades_pool().run(Job { pool: db.clone(), q }).await;
    Json(res)
}

// ----------------- Heavy compute -----------------

async fn compute_trades_analysis(pool: PgPool, q: TradesAnalysisRequest) -> TradesAnalysisResponse {
    let days = parse_period_days(&q.period);
    let since = OffsetDateTime::now_utc() - time::Duration::days(days);
    let interval_min = q.interval_min.max(1);

    // Resolve exchange id
    let exch = sqlx::query!(
        "SELECT id FROM cex_exchanges WHERE LOWER(name)=LOWER($1)",
        q.exchange
    ).fetch_optional(&pool).await.unwrap_or(None);
    if exch.is_none() {
        return TradesAnalysisResponse {
            imbalance_scatter: vec![],
            size_distribution: SizeDistribution { bins: vec![], frequencies: vec![], normal_curve: vec![] },
            notional_rankings: vec![],
            imbalance_time_series: vec![],
            velocity_heatmap: vec![],
            whale_trades: vec![],
            correlation_matrix: vec![],
            trade_z_scores: vec![],
            microstructure_stats: vec![],
        };
    }
    let exchange_id = exch.unwrap().id;

    // ---- Select markets (coins) ----
    #[derive(Debug)]
    struct Mkt { id: i32, symbol: String }
    let mut mkts: Vec<Mkt> = Vec::new();

    if let Some(list) = &q.coins {
        for sym in list {
            if let Some(row) = sqlx::query!(
                r#"
                SELECT id, symbol
                FROM cex_markets
                WHERE exchange_id=$1 AND market_type=$2 AND is_active=TRUE
                  AND UPPER(symbol)=UPPER($3)
                LIMIT 1
                "#,
                exchange_id,
                q.market_type,
                sym
            ).fetch_optional(&pool).await.unwrap_or(None) {
                mkts.push(Mkt { id: row.id, symbol: row.symbol });
            }
        }
    }

    if mkts.is_empty() {
        // Top-N by USD notional from trades in period
        let rows = sqlx::query!(
            r#"
            SELECT m.id, m.symbol, SUM(t.quote_qty)::float8 AS usd
            FROM trades t
            JOIN cex_markets m ON m.id=t.market_id
            WHERE m.exchange_id=$1 AND m.market_type=$2 AND m.is_active=TRUE
              AND t.trade_time >= $3
            GROUP BY m.id, m.symbol
            ORDER BY usd DESC NULLS LAST
            LIMIT $4
            "#,
            exchange_id,
            q.market_type,
            since,
            q.top_n
        ).fetch_all(&pool).await.unwrap_or_default();

        mkts = rows.into_iter().map(|r| Mkt { id: r.id, symbol: r.symbol }).collect();
    }

    if mkts.is_empty() {
        return TradesAnalysisResponse {
            imbalance_scatter: vec![],
            size_distribution: SizeDistribution { bins: vec![], frequencies: vec![], normal_curve: vec![] },
            notional_rankings: vec![],
            imbalance_time_series: vec![],
            velocity_heatmap: vec![],
            whale_trades: vec![],
            correlation_matrix: vec![],
            trade_z_scores: vec![],
            microstructure_stats: vec![],
        };
    }

    // ---- Fetch trades for selected markets ----
    #[derive(Clone)]
    struct TRow {
        symbol: String,
        sec: i64,
        side: String,
        price: f64,
        quote: f64,
        id: String,
    }

    let mut trades: Vec<TRow> = Vec::new();
    for m in &mkts {
        // Use COALESCE to force non-null strings → avoids Option<String> handling mismatches
        let rows = sqlx::query!(
            r#"
            SELECT
              EXTRACT(EPOCH FROM t.trade_time)::bigint           AS sec,
              COALESCE(t.side, 'Buy')                            AS side,
              t.price::float8                                    AS price,
              t.quote_qty::float8                                AS quote,
              COALESCE(t.trade_id, '')                           AS trade_id
            FROM trades t
            WHERE t.market_id=$1 AND t.trade_time >= $2
            ORDER BY t.trade_time
            "#,
            m.id,
            since
        ).fetch_all(&pool).await.unwrap_or_default();


    for r in rows {
        // `side` and `trade_id` can be NULL in the DB, so handle Option<String>
        let side = r
            .side
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Buy".to_string());

        let id = r.trade_id.unwrap_or_else(String::new);

        trades.push(TRow {
            symbol: m.symbol.clone(),
            sec: r.sec.unwrap_or(0),
            side,                          // now a String
            price: r.price.unwrap_or(0.0),
            quote: r.quote.unwrap_or(0.0),
            id,                            // now a String
        });
    }


    }

    if trades.is_empty() {
        return TradesAnalysisResponse {
            imbalance_scatter: vec![],
            size_distribution: SizeDistribution { bins: vec![], frequencies: vec![], normal_curve: vec![] },
            notional_rankings: vec![],
            imbalance_time_series: vec![],
            velocity_heatmap: vec![],
            whale_trades: vec![],
            correlation_matrix: vec![],
            trade_z_scores: vec![],
            microstructure_stats: vec![],
        };
    }

    // Group trades by symbol
    let mut by_sym: HashMap<String, Vec<TRow>> = HashMap::new();
    for t in trades.into_iter() { by_sym.entry(t.symbol.clone()).or_default().push(t); }
    for v in by_sym.values_mut() { v.sort_by_key(|r| r.sec); }

    // ---------------- Imbalance scatter + rankings ----------------
    let mut scatter: Vec<ScatterRow> = Vec::new();
    let mut rankings: Vec<NotionalRow> = Vec::new();

    for (sym, rows) in &by_sym {
        let mut buy = 0.0f64;
        let mut sell = 0.0f64;
        let mut count = 0usize;
        let mut sum_size = 0.0f64;

        for r in rows {
            let qv = finite(r.quote);
            sum_size += qv;
            count += 1;
            if r.side.eq_ignore_ascii_case("buy") { buy += qv; } else { sell += qv; }
        }
        let tot = (buy + sell).max(0.0);
        let avg_size = if count>0 { sum_size / count as f64 } else { 0.0 };
        let buy_ratio = if tot>0.0 { 100.0 * buy / tot } else { 0.0 };
        let imbalance = if tot>0.0 { 100.0 * (buy - sell) / tot } else { 0.0 };

        scatter.push(ScatterRow {
            symbol: sym.clone(),
            buy_ratio: finite(buy_ratio),
            volume: finite(tot),
            avg_trade_size: finite(avg_size),
            imbalance: finite(imbalance),
        });

        rankings.push(NotionalRow {
            symbol: sym.clone(),
            avg_size: finite(avg_size),
            interval: format!("{}m", interval_min),
        });
    }
    rankings.sort_by(|a,b| b.avg_size.total_cmp(&a.avg_size));

    // ---------------- Imbalance time series & velocity heatmap ----------------
    let mut grid_set: BTreeSet<i64> = BTreeSet::new();
    let mut per_minute_imbalance: HashMap<String, BTreeMap<i64,(f64,f64)>> = HashMap::new();
    let mut per_minute_counts: HashMap<String, BTreeMap<i64,usize>> = HashMap::new();

    for (sym, rows) in &by_sym {
        let mimb = per_minute_imbalance.entry(sym.clone()).or_default();
        let mcnt = per_minute_counts.entry(sym.clone()).or_default();
        for r in rows {
            let minute = floor_minute(r.sec, interval_min);
            grid_set.insert(minute);
            let e = mimb.entry(minute).or_insert((0.0,0.0));
            if r.side.eq_ignore_ascii_case("buy") { e.0 += finite(r.quote); } else { e.1 += finite(r.quote); }
            *mcnt.entry(minute).or_insert(0) += 1;
        }
    }
    let grid: Vec<i64> = grid_set.into_iter().collect();

    // Imbalance time series
    let mut imb_series: Vec<Series1D> = Vec::new();
    for (sym, m) in &per_minute_imbalance {
        let mut data = Vec::with_capacity(grid.len());
        for &g in &grid {
            let (b, s) = m.get(&g).copied().unwrap_or((0.0,0.0));
            let tot = (b+s).max(0.0);
            let v = if tot>0.0 { 100.0 * (b - s) / tot } else { 0.0 };
            data.push(Point { timestamp: ms(g), value: finite(v) });
        }
        imb_series.push(Series1D { symbol: sym.clone(), data });
    }

    // Velocity heatmap (trades per minute)
    let mut heatmap: Vec<Vec<f64>> = Vec::new();
    let mut symbols: Vec<String> = by_sym.keys().cloned().collect();
    symbols.sort_unstable();
    for sym in &symbols {
        let mcnt = per_minute_counts.get(sym).unwrap();
        let mut row = Vec::with_capacity(grid.len());
        for &g in &grid {
            let c = *mcnt.get(&g).unwrap_or(&0) as f64 / interval_min.max(1) as f64;
            row.push(finite(c));
        }
        heatmap.push(row);
    }

    // ---------------- Whale trades ----------------
    let mut all_sizes: Vec<f64> = Vec::new();
    for rows in by_sym.values() { for r in rows { all_sizes.push(finite(r.quote)); } }
    let thr = percentile(all_sizes.clone(), q.whale_percentile.clamp(0.9, 0.9999));

    let mut whales: Vec<WhaleRow> = Vec::new();
    for (sym, rows) in &by_sym {
        for r in rows {
            if r.quote >= thr {
                whales.push(WhaleRow {
                    id: r.id.clone(),
                    symbol: sym.clone(),
                    timestamp: ms(r.sec),
                    size: finite(r.quote),
                    price: finite(r.price),
                    side: if r.side.eq_ignore_ascii_case("buy") {"buy".into()} else {"sell".into()},
                });
            }
        }
    }
    whales.sort_by_key(|w| w.timestamp);
    if whales.len() > 100 { whales = whales.split_off(whales.len()-100); }

    // ---------------- Cross-asset correlation (counts/min) ----------------
    let mut count_series: Vec<Vec<f64>> = Vec::new();
    for sym in &symbols {
        let mcnt = per_minute_counts.get(sym).unwrap();
        let mut row = Vec::with_capacity(grid.len());
        for &g in &grid { row.push(*mcnt.get(&g).unwrap_or(&0) as f64); }
        count_series.push(row);
    }
    let n = symbols.len();
    let mut corr = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in i..n {
            let c = pearson(&count_series[i], &count_series[j]);
            corr[i][j] = c; corr[j][i] = c;
        }
    }

    // ---------------- Size distribution ----------------
    let hist_sym = q.histogram_symbol.clone().unwrap_or_else(|| {
        symbols.first().cloned().unwrap_or_else(|| "BTC".into())
    });
    let sizes: Vec<f64> = by_sym.get(&hist_sym)
        .map(|rows| rows.iter().map(|r| finite(r.quote)).collect())
        .unwrap_or_else(|| Vec::new());

    let size_dist = if sizes.is_empty() {
        SizeDistribution { bins: vec![], frequencies: vec![], normal_curve: vec![] }
    } else {
        let min_s = sizes.iter().copied().fold(f64::INFINITY, f64::min).max(1.0);
        let max_s = sizes.iter().copied().fold(0.0, f64::max).max(min_s*1.1);
        let min_e = min_s.log10().floor();
        let max_e = max_s.log10().ceil();
        let mut bins = Vec::new();
        let mut e = min_e;
        while e <= max_e + 0.0001 { bins.push(10f64.powf(e)); e += 0.25; }
        if bins.len() < 2 { bins.push(bins[0]*1.5); }

        let mut freq = vec![0usize; bins.len()-1];
        for &x in &sizes {
            for i in 0..freq.len() {
                let l=bins[i]; let r=bins[i+1]; let last=i==freq.len()-1;
                if (x>=l && x<r) || (last && x==r) { freq[i]+=1; break; }
            }
        }

        let logs: Vec<f64> = sizes.iter().filter(|&&v| v>0.0).map(|&v| v.ln()).collect();
        let mu = mean(&logs);
        let sigma = stddev(&logs).max(1e-6);
        let total = freq.iter().sum::<usize>().max(1) as f64;

        let mut curve = Vec::with_capacity(freq.len());
        for i in 0..freq.len() {
            let mid = (bins[i]+bins[i+1])*0.5;
            let pdf = lognormal_pdf(mid, mu, sigma);
            let width = (bins[i+1]-bins[i]).max(1.0);
            curve.push(finite(pdf * total * width));
        }

        SizeDistribution { bins, frequencies: freq, normal_curve: curve }
    };

    // ---------------- Trade Z-scores (last ~1h) ----------------
    let lookback = 60 / interval_min.max(1) as usize;
    let mut zrows: Vec<ZRow> = Vec::new();

    for sym in &symbols {
        let mcnt = per_minute_counts.get(sym).unwrap();
        let mut counts = Vec::with_capacity(grid.len());
        let mut avgsz  = Vec::with_capacity(grid.len());
        let mut imbs   = Vec::with_capacity(grid.len());

        let mimb = per_minute_imbalance.get(sym).unwrap();
        for &g in &grid {
            let c = *mcnt.get(&g).unwrap_or(&0) as f64;
            counts.push(c);

            let (b,s) = mimb.get(&g).copied().unwrap_or((0.0,0.0));
            let tot = (b+s).max(0.0);
            avgsz.push(if c>0.0 { tot/c } else { 0.0 });

            let imb = if tot>0.0 { 100.0*(b-s)/tot } else { 0.0 };
            imbs.push(imb);
        }

        let tail = |v: &Vec<f64>| -> f64 {
            if v.is_empty() { 0.0 } else {
                let k = v.len().min(lookback.max(1));
                mean(&v[v.len()-k..])
            }
        };

        let cur_count = tail(&counts);
        let cur_size  = tail(&avgsz);
        let cur_imb   = tail(&imbs);

        let z = |v: &Vec<f64>, cur: f64| -> (f64,f64,f64) {
            let mu = mean(v);
            let sd = stddev(v);
            (finite(cur), finite(mu), if sd>0.0 { finite((cur-mu)/sd) } else { 0.0 })
        };

        let (c_cur, c_mu, c_z) = z(&counts, cur_count);
        let (s_cur, s_mu, s_z) = z(&avgsz,  cur_size);
        let (i_cur, i_mu, i_z) = z(&imbs,   cur_imb);

        zrows.push(ZRow { name: format!("{}: trade count", sym), kind: "count".into(),   current: c_cur, average: c_mu, zscore: c_z });
        zrows.push(ZRow { name: format!("{}: avg size",   sym), kind: "size".into(),    current: s_cur, average: s_mu, zscore: s_z });
        zrows.push(ZRow { name: format!("{}: imbalance",  sym), kind: "percent".into(), current: i_cur, average: i_mu, zscore: i_z });
    }

    // ---------------- Microstructure stats (aggregate) ----------------
    let mut stats: Vec<StatRow> = Vec::new();
    let total_usd: f64 = scatter.iter().map(|r| r.volume).sum();
    let total_trades: usize = by_sym.values().map(|v| v.len()).sum();
    let overall_avg = if total_trades>0 { total_usd / total_trades as f64 } else { 0.0 };
    stats.push(StatRow { name: "Total USD (period)".into(), value: format!("{:.0}", total_usd) });
    stats.push(StatRow { name: "Total trades (period)".into(), value: format!("{}", total_trades) });
    stats.push(StatRow { name: "Overall avg trade size".into(), value: format!("{:.2}", overall_avg) });
    let thr = percentile(all_sizes.clone(), q.whale_percentile.clamp(0.9, 0.99));
    stats.push(StatRow { name: "Whale threshold (USD)".into(), value: format!("{:.0}", thr) });
    stats.push(StatRow { name: "Assets".into(), value: format!("{}", symbols.len()) });

    TradesAnalysisResponse {
        imbalance_scatter: scatter,
        size_distribution: size_dist,
        notional_rankings: rankings,
        imbalance_time_series: imb_series,
        velocity_heatmap: heatmap,
        whale_trades: whales,
        correlation_matrix: corr,
        trade_z_scores: zrows,
        microstructure_stats: stats,
    }
}
