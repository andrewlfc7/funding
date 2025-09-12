// backend/src/api/statistics/market_seasonality.rs

use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{Error as DeError, SeqAccess, Visitor};
use sqlx::PgPool;
use std::{collections::{BTreeMap, BTreeSet}, fmt, sync::OnceLock};

use crate::infra::task_pools::{EndpointPool, threads_from_env};

use super::{
    fetch_multi_hourly_ohlcv, parse_period_days, resample_from_hourly,
    top_markets_by_usd_volume_live, Tf,
};

// ---------- Config & utils ----------

fn default_market_type() -> String { "spot".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }

fn string_or_seq<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where D: Deserializer<'de> {
    struct StrOrSeq;
    impl<'de> Visitor<'de> for StrOrSeq {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "string, CSV, or sequence") }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: DeError {
            Ok(v.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).map(String::from).collect())
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
            let mut out = Vec::new();
            while let Some(elem) = seq.next_element::<String>()? {
                out.extend(elem.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).map(String::from));
            }
            Ok(out)
        }
    }
    de.deserialize_any(StrOrSeq)
}

fn is_stable_symbol(sym: &str) -> bool {
    let s = sym.trim().to_ascii_uppercase();
    const STABLES: &[&str] = &[
        "USD","USDT","USDC","FDUSD","FUSD","TUSD","BUSD","DAI","PYUSD","USDE","USDD","USDP",
        "GUSD","USDJ","USDX","FRAX","LUSD","SUSD","MIM","DOLA","EURS","EURC","EURT","CRVUSD","USDL",
    ];
    if STABLES.contains(&s.as_str()) { return true; }
    s.ends_with("USD") || s.ends_with("USDT") || s.ends_with("USDC")
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len(); if n == 0 || n != y.len() { return 0.0; }
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0,0.0,0.0,0.0,0.0);
    let mut m = 0usize;
    for i in 0..n {
        let (xi, yi) = (x[i], y[i]);
        if xi.is_finite() && yi.is_finite() { sx+=xi; sy+=yi; sxx+=xi*xi; syy+=yi*yi; sxy+=xi*yi; m+=1; }
    }
    if m <= 1 { return 0.0; }
    let mf = m as f64;
    let cov = sxy - sx*sy/mf;
    let vx  = sxx - sx*sx/mf;
    let vy  = syy - sy*sy/mf;
    if vx <= 0.0 || vy <= 0.0 { return 0.0; }
    finite(cov / (vx.sqrt()*vy.sqrt()))
}

fn autocorr(v: &[f64], lag: usize) -> f64 {
    if lag == 0 || v.len() <= lag { return 0.0; }
    let x = &v[lag..];
    let y = &v[..v.len()-lag];
    pearson(x, y)
}

fn hour_of(ts: i64) -> Option<u8> {
    time::OffsetDateTime::from_unix_timestamp(ts).ok().map(|dt| dt.hour())
}
fn dow_of(ts: i64) -> Option<u8> {
    // 0=Mon .. 6=Sun
    time::OffsetDateTime::from_unix_timestamp(ts).ok().map(|dt| dt.weekday().number_from_monday() as u8 - 1)
}
fn month_of(ts: i64) -> Option<u8> {
    time::OffsetDateTime::from_unix_timestamp(ts).ok().map(|dt| dt.month() as u8 - 1) // 0..11
}

// ---------- Request / Response DTOs ----------

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSeasonalityRequest {
    pub exchange: String,
    pub period: String,
    #[serde(default = "default_market_type")]
    pub market_type: String,
    #[serde(default = "default_timeframe")]
    pub timeframe: String, // 1h|4h|1d
    #[serde(default)]
    pub coins: Option<Vec<String>>,
    #[serde(default)]
    pub top_n: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketSeasonalityResponse {
    pub intraday_heatmap: Vec<Vec<f64>>, // [assets][hours 0-23] avg vol %
    pub weekday_heatmap: WeekdayHeatmap,
    pub monthly_seasonality: BTreeMap<String, MonthlySeasonality>,
    pub volume_autocorrelation: BTreeMap<String, Vec<f64>>, // lags 1..24
    pub anomalies: Vec<Anomaly>,
    pub volume_persistence: Vec<VolumePersistence>,
    pub most_active_periods: Vec<ActivePeriod>,
    pub least_active_periods: Vec<ActivePeriod>,
    pub strongest_patterns: Vec<StrongPattern>,
    pub volatility_clusters: Vec<VolCluster>,
    pub weekend_effect: Vec<WeekendEffect>,
    pub timezone_effects: Vec<TimezoneEffect>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekdayHeatmap {
    pub volatility: Vec<Vec<f64>>, // [assets][0..6] %
    pub volume: Vec<Vec<f64>>,     // [assets][0..6] USD
    pub returns: Vec<Vec<f64>>,    // [assets][0..6] %
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlySeasonality {
    pub returns: Vec<f64>,     // 12
    pub volatility: Vec<f64>,  // 12
}

#[derive(Debug, Serialize)]
pub struct Anomaly {
    pub symbol: String,
    pub period: String,   // "Mon 14:00"
    pub metric: String,   // "volatility" | "volume"
    pub current: f64,
    pub historical: f64,
    pub zscore: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumePersistence {
    pub symbol: String,
    pub lag1h: f64,
    pub lag4h: f64,
    pub lag24h: f64,
    pub pattern: String,  // "Persistent" | "Mean-reverting" | "Neutral"
}

#[derive(Debug, Serialize)]
pub struct ActivePeriod {
    pub id: String,
    pub asset: String,
    pub period: String,   // "Mon 00:00-24:00 UTC"
    pub volatility: f64,  // %
    pub volume: f64,      // USD
}

#[derive(Debug, Serialize)]
pub struct StrongPattern {
    pub id: String,
    pub name: String,     // "US Market Open Spike"
    pub strength: f64,    // 0..100
}

#[derive(Debug, Serialize)]
pub struct VolCluster {
    pub id: String,
    pub period: String,   // "14:00-16:00 UTC"
    pub assets: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct WeekendEffect {
    pub asset: String,
    pub effect: f64,      // % diff vs weekday avg
}

#[derive(Debug, Serialize)]
pub struct TimezoneEffect {
    pub zone: String,         // "US" | "EU" | "ASIA"
    pub active_hours: String, // "14:00-22:00 UTC"
    pub impact: String,       // "High volatility" etc.
}

// ---------- Handler via task pool ----------

#[derive(Clone)]
struct Job { pool: PgPool, q: MarketSeasonalityRequest }

static SEASON_POOL: OnceLock<EndpointPool<Job, MarketSeasonalityResponse>> = OnceLock::new();

fn pool() -> &'static EndpointPool<Job, MarketSeasonalityResponse> {
    SEASON_POOL.get_or_init(|| {
        let n = threads_from_env("MARKET_SEASONALITY_THREADS", 2);
        EndpointPool::start("market-seasonality", n, |job: Job| async move {
            compute_market_seasonality(job.pool, job.q).await
        })
    })
}

pub async fn get_market_seasonality(
    State(db): State<PgPool>,
    Query(q): Query<MarketSeasonalityRequest>,
) -> Json<MarketSeasonalityResponse> {
    let res = pool().run(Job { pool: db.clone(), q }).await;
    Json(res)
}

// ---------- Heavy computation ----------

async fn compute_market_seasonality(
    pool: PgPool,
    q: MarketSeasonalityRequest,
) -> MarketSeasonalityResponse {
    // 1) Universe
    let days = parse_period_days(&q.period);
    let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();

    let mut symset: BTreeSet<String> =
        q.coins.clone().unwrap_or_default().into_iter().map(|s| s.to_uppercase()).collect();

    if let Some(n) = q.top_n {
        if let Ok(top) = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.market_type, days as i32, n).await {
            symset.extend(top.into_iter().map(|(s, _, _)| s.to_uppercase()));
        }
    }

    // Filter out stables
    symset = symset.into_iter().filter(|s| !is_stable_symbol(s)).collect();

    let mut syms: Vec<String> = symset.into_iter().collect();
    syms.sort_unstable();

    if syms.is_empty() {
        return empty_response();
    }

    // 2) Resolve market ids and pull hourly raw, then resample to requested TF
    let mut mids = Vec::new(); let mut sym_mid = Vec::new();
    for s in &syms {
        if let Ok(mid) = super::resolve_market_id_with_data(&pool, &q.exchange, s, &q.market_type, since_unix).await {
            mids.push(mid); sym_mid.push((s.clone(), mid));
        }
    }
    if sym_mid.is_empty() { return empty_response(); }

    // *** FIX: Create a definitive list of symbols for which we have data. ***
    let syms_with_data: Vec<String> = sym_mid.iter().map(|(s, _)| s.clone()).collect();

    let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix).await.unwrap_or_default();
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);

    // 3) Accumulators
    let mut intraday: Vec<[f64; 24]> = Vec::new();
    let mut intraday_cnt: Vec<[usize; 24]> = Vec::new();

    let mut weekday_vol: Vec<[f64; 7]> = Vec::new();
    let mut weekday_vol_cnt: Vec<[usize; 7]> = Vec::new();
    let mut weekday_vol_usd: Vec<[f64; 7]> = Vec::new();
    let mut weekday_vol_usd_cnt: Vec<[usize; 7]> = Vec::new();
    let mut weekday_ret: Vec<[f64; 7]> = Vec::new();
    let mut weekday_ret_cnt: Vec<[usize; 7]> = Vec::new();

    let mut monthly_ret: BTreeMap<String, [f64; 12]> = BTreeMap::new();
    let mut monthly_ret_cnt: BTreeMap<String, [usize; 12]> = BTreeMap::new();
    let mut monthly_vol: BTreeMap<String, [f64; 12]> = BTreeMap::new();
    let mut monthly_vol_cnt: BTreeMap<String, [usize; 12]> = BTreeMap::new();

    let mut vol_acf: BTreeMap<String, Vec<f64>> = BTreeMap::new();

    // For anomalies & activity
    let mut latest_bars: BTreeMap<String, (i64, f64, f64, f64)> = BTreeMap::new(); // ts, range%, usd vol, ret%
    let mut per_symbol_hour_dists: BTreeMap<String, Vec<(u8, u8, f64, f64)>> = BTreeMap::new(); // (dow,hour, meanVol%, stdVol%)

    // 4) Per-symbol compute
    for (sym, mid) in &sym_mid {
        let Some(h) = by_mid.get(mid) else { continue; };
        let ser = resample_from_hourly(h, tf.period_secs());
        if ser.len() < 20 { continue; }

        let mut hours = [0.0f64; 24];
        let mut hours_c = [0usize; 24];

        let mut dows = [0.0f64; 7];
        let mut dows_c = [0usize; 7];

        let mut dows_volusd = [0.0f64; 7];
        let mut dows_volusd_c = [0usize; 7];

        let mut dows_ret = [0.0f64; 7];
        let mut dows_ret_c = [0usize; 7];

        let mut mret = [0.0f64; 12];
        let mut mret_c = [0usize; 12];
        let mut mvol = [0.0f64; 12];
        let mut mvol_c = [0usize; 12];

        let mut usd_vec = Vec::with_capacity(ser.len());
        let mut ret_vec = Vec::with_capacity(ser.len());
        let mut rng_vec = Vec::with_capacity(ser.len());

        for i in 0..ser.len() {
            let r = &ser[i];
            let ts = r.ts;
            let close = r.close;
            let high  = r.high;
            let low   = r.low;
            let base  = r.volume;
            let usd   = finite(close * base);
            let range_pct = if low > 0.0 { finite((high - low) / low * 100.0) } else { 0.0 };
            let ret_pct = if i > 0 && ser[i-1].close > 0.0 {
                finite(((close / ser[i-1].close) - 1.0) * 100.0)
            } else { 0.0 };

            // Intraday
            if let Some(hh) = hour_of(ts) {
                hours[hh as usize] += range_pct;
                hours_c[hh as usize] += 1;
            }
            // Weekday
            if let Some(dw) = dow_of(ts) {
                dows[dw as usize] += range_pct;
                dows_c[dw as usize] += 1;
                dows_volusd[dw as usize] += usd;
                dows_volusd_c[dw as usize] += 1;
                dows_ret[dw as usize] += ret_pct;
                dows_ret_c[dw as usize] += 1;
            }
            // Monthly
            if let Some(mo) = month_of(ts) {
                mret[mo as usize] += ret_pct; mret_c[mo as usize] += 1;
                mvol[mo as usize] += range_pct; mvol_c[mo as usize] += 1;
            }

            usd_vec.push(usd);
            ret_vec.push(ret_pct);
            rng_vec.push(range_pct);
        }

        // Save latest for anomalies
        let last = ser.last().unwrap();
        let last_range = if last.low > 0.0 { (last.high - last.low) / last.low * 100.0 } else { 0.0 };
        let last_usd   = finite(last.close * last.volume);
        let last_ret   = if ser.len()>1 && ser[ser.len()-2].close>0.0 {
            ((last.close / ser[ser.len()-2].close) - 1.0) * 100.0
        } else { 0.0 };
        latest_bars.insert(sym.clone(), (last.ts, finite(last_range), finite(last_usd), finite(last_ret)));

        // Hourly distributions per weekday for anomaly z-scores
        let mut dist: Vec<(u8,u8,f64,f64)> = Vec::new();
        for dow in 0..7u8 {
            for hour in 0..24u8 {
                // collect ranges for matching (dow,hour)
                let mut vals = Vec::new();
                for i in 0..ser.len() {
                    if let (Some(d), Some(hh)) = (dow_of(ser[i].ts), hour_of(ser[i].ts)) {
                        if d==dow && hh==hour {
                            let r = &ser[i];
                            let v = if r.low>0.0 { (r.high - r.low)/r.low*100.0 } else { 0.0 };
                            if v.is_finite() { vals.push(v); }
                        }
                    }
                }
                if vals.len() >= 6 {
                    let mean = vals.iter().copied().sum::<f64>() / (vals.len() as f64);
                    let var  = vals.iter().map(|&x| { let d=x-mean; d*d }).sum::<f64>() / (vals.len() as f64);
                    let sd   = var.sqrt();
                    dist.push((dow, hour, finite(mean), finite(sd)));
                }
            }
        }
        per_symbol_hour_dists.insert(sym.clone(), dist);

        // Aggregate collectors
        intraday.push(hours);
        intraday_cnt.push(hours_c);

        weekday_vol.push(dows);
        weekday_vol_cnt.push(dows_c);
        weekday_vol_usd.push(dows_volusd);
        weekday_vol_usd_cnt.push(dows_volusd_c);
        weekday_ret.push(dows_ret);
        weekday_ret_cnt.push(dows_ret_c);

        monthly_ret.insert(sym.clone(), mret);
        monthly_ret_cnt.insert(sym.clone(), mret_c);
        monthly_vol.insert(sym.clone(), mvol);
        monthly_vol_cnt.insert(sym.clone(), mvol_c);

        // Volume ACF 1..24
        let mut acf = Vec::with_capacity(24);
        for lag in 1..=24 {
            acf.push(finite(autocorr(&usd_vec, lag)));
        }
        vol_acf.insert(sym.clone(), acf);
    }

    // 5) Reduce to averages
    let mut intraday_heatmap: Vec<Vec<f64>> = Vec::new();
    for i in 0..intraday.len() {
        let mut row = vec![0.0; 24];
        for h in 0..24 {
            let c = intraday_cnt[i][h];
            row[h] = if c > 0 { intraday[i][h] / c as f64 } else { 0.0 };
        }
        intraday_heatmap.push(row);
    }

    let mut weekday_volatility: Vec<Vec<f64>> = Vec::new();
    let mut weekday_volume: Vec<Vec<f64>> = Vec::new();
    let mut weekday_returns: Vec<Vec<f64>> = Vec::new();
    for i in 0..weekday_vol.len() {
        let mut v1 = vec![0.0;7];
        let mut v2 = vec![0.0;7];
        let mut v3 = vec![0.0;7];
        for d in 0..7 {
            v1[d] = if weekday_vol_cnt[i][d]>0 { weekday_vol[i][d] / weekday_vol_cnt[i][d] as f64 } else { 0.0 };
            v2[d] = if weekday_vol_usd_cnt[i][d]>0 { weekday_vol_usd[i][d] / weekday_vol_usd_cnt[i][d] as f64 } else { 0.0 };
            v3[d] = if weekday_ret_cnt[i][d]>0 { weekday_ret[i][d] / weekday_ret_cnt[i][d] as f64 } else { 0.0 };
        }
        weekday_volatility.push(v1);
        weekday_volume.push(v2);
        weekday_returns.push(v3);
    }

    let mut monthly_seasonality: BTreeMap<String, MonthlySeasonality> = BTreeMap::new();
    for (sym, arr) in monthly_ret {
        let cnt = monthly_ret_cnt.get(&sym).unwrap();
        let vol = monthly_vol.get(&sym).unwrap();
        let volcnt = monthly_vol_cnt.get(&sym).unwrap();
        let mut retv = vec![0.0;12];
        let mut volv = vec![0.0;12];
        for m in 0..12 {
            retv[m] = if cnt[m]>0 { arr[m] / cnt[m] as f64 } else { 0.0 };
            volv[m] = if volcnt[m]>0 { vol[m] / volcnt[m] as f64 } else { 0.0 };
        }
        monthly_seasonality.insert(sym, MonthlySeasonality { returns: retv, volatility: volv });
    }

    // 6) Anomalies (volatility & volume)
    let mut anomalies: Vec<Anomaly> = Vec::new();
    for (sym, (ts, last_range, last_usd, _)) in &latest_bars {
        if let (Some(dw), Some(hh)) = (dow_of(*ts), hour_of(*ts)) {
            if let Some(dists) = per_symbol_hour_dists.get(sym) {
                if let Some((_,_,m,sd)) = dists.iter().find(|(d,h,_,_)| *d==dw && *h==hh) {
                    if *sd > 0.0 {
                        let z = (last_range - *m) / *sd;
                        if z.abs() >= 2.0 {
                            anomalies.push(Anomaly {
                                symbol: sym.clone(),
                                period: format!("{} {:02}:00", dow_to_str(dw), hh),
                                metric: "volatility".into(),
                                current: *last_range,
                                historical: *m,
                                zscore: finite(z),
                            });
                        }
                    }
                }
            }
        }
        // crude volume anomaly vs weekday avg
        if let (Some(dw), Some(hh)) = (dow_of(*ts), hour_of(*ts)) {
            // *** FIX: Use syms_with_data to find the correct index. ***
            let idx = syms_with_data.iter().position(|s| s == sym).unwrap_or(0);
            if let Some(wrow) = weekday_volume.get(idx) {
                let hist = wrow[dw as usize];
                if hist > 0.0 {
                    let sd = hist * 0.5;
                    if sd > 0.0 {
                        let z = (*last_usd - hist) / sd;
                        if z.abs() >= 2.0 {
                            anomalies.push(Anomaly {
                                symbol: sym.clone(),
                                period: format!("{} {:02}:00", dow_to_str(dw), hh),
                                metric: "volume".into(),
                                current: *last_usd,
                                historical: hist,
                                zscore: finite(z),
                            });
                        }
                    }
                }
            }
        }
    }
    anomalies.sort_by(|a,b| b.zscore.abs().total_cmp(&a.zscore.abs()));
    if anomalies.len() > 12 { anomalies.truncate(12); }

    // 7) Volume persistence (lag1/4/24)
    let mut volume_persistence: Vec<VolumePersistence> = Vec::new();
    // *** FIX: Iterate over syms_with_data. ***
    for sym in &syms_with_data {
        if let Some(acf) = vol_acf.get(sym) {
            let l1  = acf.get(0).copied().unwrap_or(0.0);
            let l4  = acf.get(3).copied().unwrap_or(0.0);
            let l24 = acf.get(23).copied().unwrap_or(0.0);
            let pattern = if l1 > 0.5 && l24 > 0.2 { "Persistent" }
                          else if l1 < 0.0 && l4 < 0.0 { "Mean-reverting" }
                          else { "Neutral" };
            volume_persistence.push(VolumePersistence {
                symbol: sym.clone(),
                lag1h: finite(l1),
                lag4h: finite(l4),
                lag24h: finite(l24),
                pattern: pattern.into(),
            });
        }
    }

    let mut most_active_periods: Vec<ActivePeriod> = Vec::new();
    let mut least_active_periods: Vec<ActivePeriod> = Vec::new();
    // *** FIX: Iterate over syms_with_data. ***
    for (i, sym) in syms_with_data.iter().enumerate() {
        let wv = &weekday_volatility[i];
        let wvu = &weekday_volume[i];

        let (zv, zvu) = (zvec(wv), zvec(wvu));
        let mut rows: Vec<(usize, f64, f64)> = (0..7).map(|d| (d, zv[d] + zvu[d], wv[d])).collect();
        rows.sort_by(|a,b| b.1.total_cmp(&a.1));
        for (rank, &(d, _score, volpct)) in rows.iter().take(3).enumerate() {
            most_active_periods.push(ActivePeriod {
                id: format!("{}-top-{}", sym, rank+1),
                asset: sym.clone(),
                period: format!("{} 00:00-24:00 UTC", dow_to_str(d as u8)),
                volatility: finite(volpct),
                volume: finite(wvu[d]),
            });
        }
        rows.sort_by(|a,b| a.1.total_cmp(&b.1));
        for (rank, &(d, _score, volpct)) in rows.iter().take(3).enumerate() {
            least_active_periods.push(ActivePeriod {
                id: format!("{}-low-{}", sym, rank+1),
                asset: sym.clone(),
                period: format!("{} 00:00-24:00 UTC", dow_to_str(d as u8)),
                volatility: finite(volpct),
                volume: finite(wvu[d]),
            });
        }
    }

    // 9) Strongest patterns (coarse heuristic)
    let patterns = vec![
        ("US Market Open Spike", "13:00-16:00 UTC"),
        ("EU Session Activity",  "07:00-10:00 UTC"),
        ("Asia Session Burst",   "00:00-03:00 UTC"),
        ("Weekend Compression",  "Sat-Sun"),
    ];
    let mut strongest_patterns = Vec::new();
    for (idx, (name, hours)) in patterns.iter().enumerate() {
        let (start_h, end_h, is_weekend) = parse_window(hours);
        let mut hits = 0usize;
        for row in &intraday_heatmap {
            let med = median(row);
            let win_avg = if is_weekend {
                // simple baseline for weekend: use the row median
                med
            } else {
                avg_range(row, start_h, end_h)
            };
            if win_avg > med { hits += 1; }
        }
        let strength = if !intraday_heatmap.is_empty() {
            (hits as f64) / (intraday_heatmap.len() as f64) * 100.0
        } else { 0.0 };
        strongest_patterns.push(StrongPattern {
            id: format!("pat-{}", idx+1),
            name: name.to_string(),
            strength: finite(strength),
        });
    }

    // 10) Volatility clusters (hours where many assets > asset 75th pct intraday)
    let mut vol_clusters = Vec::new();
    if !intraday_heatmap.is_empty() {
        let mut above: [usize; 24] = [0;24];
        let mut assets_above: [Vec<String>; 24] = Default::default();
        for (i, row) in intraday_heatmap.iter().enumerate() {
            let thr = percentile(row, 75.0);
            for h in 0..24 {
                // *** FIX: Use syms_with_data to get the correct symbol. ***
                if row[h] > thr { above[h] += 1; assets_above[h].push(syms_with_data[i].clone()); }
            }
        }
        let k = ((syms_with_data.len() as f64) * 0.4).ceil() as usize;
        let mut h=0;
        while h<24 {
            if above[h] >= k {
                let start = h;
                let mut aset: BTreeSet<String> = assets_above[h].iter().cloned().collect();
                h+=1;
                while h<24 && above[h] >= k {
                    for s in &assets_above[h] { aset.insert(s.clone()); }
                    h+=1;
                }
                vol_clusters.push(VolCluster {
                    id: format!("h{:02}-h{:02}", start, (h-1)),
                    period: format!("{:02}:00-{:02}:00 UTC", start, h%24),
                    assets: aset.into_iter().collect(),
                });
            } else { h+=1; }
        }
    }

    // 11) Weekend effect
    let mut weekend_effect = Vec::new();
    // *** FIX: Iterate over syms_with_data. ***
    for (i, sym) in syms_with_data.iter().enumerate() {
        let wk = 0.5 * (weekday_volatility[i][5] + weekday_volatility[i][6]);
        let wd = (0..5).map(|d| weekday_volatility[i][d]).sum::<f64>() / 5.0;
        let eff = if wd>0.0 { (wk - wd)/wd * 100.0 } else { 0.0 };
        weekend_effect.push(WeekendEffect { asset: sym.clone(), effect: finite(eff) });
    }

    // 12) Timezone effects (coarse)
    let zones = vec![
        ("US",   "13:00-21:00 UTC"),
        ("EU",   "07:00-15:00 UTC"),
        ("ASIA", "23:00-07:00 UTC"),
    ];
    let mut timezone_effects = Vec::new();
    for (zone, win) in zones {
        let (s,e,wrap) = parse_hours(win);
        let mut m = 0.0;
        for row in &intraday_heatmap {
            m += avg_range_wrap(row, s, e, wrap);
        }
        if !intraday_heatmap.is_empty() {
            m /= intraday_heatmap.len() as f64;
        }
        let impact = if m > 1.5 { "High volatility" }
                     else if m > 0.8 { "Moderate volatility" }
                     else { "Low volatility" };
        timezone_effects.push(TimezoneEffect { zone: zone.into(), active_hours: win.into(), impact: impact.into() });
    }

    MarketSeasonalityResponse {
        intraday_heatmap,
        weekday_heatmap: WeekdayHeatmap {
            volatility: weekday_volatility,
            volume: weekday_volume,
            returns: weekday_returns,
        },
        monthly_seasonality,
        volume_autocorrelation: vol_acf,
        anomalies,
        volume_persistence,
        most_active_periods,
        least_active_periods,
        strongest_patterns,
        volatility_clusters: vol_clusters,
        weekend_effect,
        timezone_effects,
    }
}

// ---------- Helpers for response defaults & math ----------

fn empty_response() -> MarketSeasonalityResponse {
    MarketSeasonalityResponse {
        intraday_heatmap: vec![],
        weekday_heatmap: WeekdayHeatmap { volatility: vec![], volume: vec![], returns: vec![] },
        monthly_seasonality: BTreeMap::new(),
        volume_autocorrelation: BTreeMap::new(),
        anomalies: vec![],
        volume_persistence: vec![],
        most_active_periods: vec![],
        least_active_periods: vec![],
        strongest_patterns: vec![],
        volatility_clusters: vec![],
        weekend_effect: vec![],
        timezone_effects: vec![],
    }
}

fn zvec(v: &[f64]) -> Vec<f64> {
    if v.is_empty() { return vec![]; }
    let mean = v.iter().copied().sum::<f64>() / v.len() as f64;
    let var  = v.iter().map(|&x| { let d=x-mean; d*d }).sum::<f64>() / v.len() as f64;
    let sd   = var.sqrt();
    v.iter().map(|&x| if sd>0.0 { (x-mean)/sd } else { 0.0 }).map(finite).collect()
}

fn median(v: &[f64]) -> f64 {
    if v.is_empty() { return 0.0; }
    let mut a = v.to_vec();
    a.sort_by(|x,y| x.total_cmp(y));
    let n=a.len();
    if n%2==1 { a[n/2] } else { 0.5*(a[n/2-1]+a[n/2]) }
}

fn percentile(v: &[f64], p: f64) -> f64 {
    if v.is_empty() { return 0.0; }
    let mut a = v.to_vec();
    a.sort_by(|x,y| x.total_cmp(y));
    let idx = ((p/100.0) * ((a.len()-1) as f64)).round() as usize;
    if idx < a.len() {
        a[idx]
    } else {
        a.last().copied().unwrap_or(0.0)
    }
}

fn avg_range(row: &[f64], s: usize, e: usize) -> f64 {
    if s<=e {
        let mut sum=0.0; let mut c=0.0;
        for h in s..=e { if h < row.len() { sum+=row[h]; c+=1.0; } }
        if c>0.0 { sum/c } else { 0.0 }
    } else { 0.0 }
}

fn parse_hours(win: &str) -> (usize, usize, bool) {
    // "HH:MM-HH:MM UTC"
    let parts: Vec<&str> = win.split(' ').next().unwrap_or("").split('-').collect();
    let h1 = parts.get(0).and_then(|s| s.split(':').next()).and_then(|h| h.parse::<usize>().ok()).unwrap_or(0);
    let h2 = parts.get(1).and_then(|s| s.split(':').next()).and_then(|h| h.parse::<usize>().ok()).unwrap_or(0);
    (h1, h2, h2 < h1)
}

fn avg_range_wrap(row: &[f64], s: usize, e: usize, wrap: bool) -> f64 {
    if !wrap { return avg_range(row, s, e); }
    let mut vals = Vec::new();
    for h in s..24 { if h < row.len() { vals.push(row[h]); } }
    for h in 0..=e { if h < row.len() { vals.push(row[h]); } }
    if vals.is_empty() { 0.0 } else { vals.iter().sum::<f64>() / vals.len() as f64 }
}

fn dow_to_str(d: u8) -> &'static str {
    match d { 0=>"Mon",1=>"Tue",2=>"Wed",3=>"Thu",4=>"Fri",5=>"Sat",_=>"Sun" }
}

fn parse_window(s: &str) -> (usize, usize, bool) {
    if s.contains("Sat-Sun") { return (0,0,true); }
    parse_hours(s)
}
