use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{Error as DeError, SeqAccess, Visitor};
use sqlx::PgPool;
use std::{collections::BTreeMap, fmt};

use super::{
    fetch_multi_hourly_ohlcv, parse_period_days, resample_from_hourly, top_markets_by_usd_volume_live,
    zscore_series, Tf,
};

fn default_market_type() -> String { "spot".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
fn default_window() -> usize { 48 }                   // 2 days @1h
fn default_vel_win() -> usize { 6 }                   // 6 periods change
#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }

/* accept "A,B" or repeat keys */
fn string_or_seq<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where D: Deserializer<'de> {
    struct StrOrSeq;
    impl<'de> Visitor<'de> for StrOrSeq {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "string/CSV/sequence") }
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

/* ---------- request / response DTOs ---------- */

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelativeStrengthRequest {
    pub period: String,
    pub exchange: String,
    #[serde(default = "default_market_type")]
    pub market_type: String,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    #[serde(default)]
    pub top_n: Option<i64>,
    #[serde(default)]
    pub compare_coins: Option<Vec<String>>,
    #[serde(default)]
    pub base_coin: Option<String>,     // default BTC
    #[serde(default = "default_window")]
    pub window: usize,                 // z-score window
    #[serde(default = "default_vel_win")]
    pub velocity_window: usize,        // Δz lookback for velocity/factor
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelativeStrengthResponse {
    pub base: String,
    pub rs_rankings: Vec<RsRank>,
    pub persistence: Vec<PersistenceRow>,
    pub momentum_factor_loadings: Vec<FactorLoading>,
    pub rs_series: Vec<RsSeries>,        // per-symbol z of ln(P_i/P_base)
    pub pair_divergence: Vec<PairDiv>,   // top by |last divergence|
}

#[derive(Debug, Serialize)]
pub struct RsRank { pub pair: String, pub last_z: f64 }
#[derive(Debug, Serialize)]
pub struct PersistenceRow { pub symbol: String, pub rho1: f64 }
#[derive(Debug, Serialize)]
pub struct FactorLoading { pub symbol: String, pub beta: f64, pub r2: f64 }

#[derive(Debug, Serialize)]
pub struct RsSeries { pub symbol: String, pub ts: Vec<i64>, pub z: Vec<f64> }

#[derive(Debug, Serialize)]
pub struct PairDiv { pub pair: String, pub time_series: Vec<DivPoint> }
#[derive(Debug, Serialize)]
pub struct DivPoint { pub timestamp: i64, pub z1: f64, pub z2: f64, pub divergence: f64 }

/* ---------- stats helpers ---------- */

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len()); if n == 0 { return 0.0; }
    let (mut sx,mut sy,mut sxx,mut syy,mut sxy)=(0.0,0.0,0.0,0.0,0.0); let mut m=0usize;
    for i in 0..n { let (a,b)=(x[i],y[i]); if a.is_finite() && b.is_finite(){ sx+=a; sy+=b; sxx+=a*a; syy+=b*b; sxy+=a*b; m+=1; } }
    if m<=1 { return 0.0; }
    let mf=m as f64; let cov=sxy - sx*sy/mf; let vx=sxx - sx*sx/mf; let vy=syy - sy*sy/mf;
    if vx<=0.0 || vy<=0.0 { 0.0 } else { finite(cov/(vx.sqrt()*vy.sqrt())) }
}

fn beta_r2(x: &[f64], f: &[f64]) -> (f64, f64) {
    let n = x.len().min(f.len()); if n == 0 { return (0.0, 0.0); }
    let (mut sx,mut sf,mut sxx,mut sff,mut sxf)=(0.0,0.0,0.0,0.0,0.0); let mut m=0usize;
    for i in 0..n {
        let (a,b)=(x[i],f[i]);
        if a.is_finite() && b.is_finite(){ sx+=a; sf+=b; sxx+=a*a; sff+=b*b; sxf+=a*b; m+=1; }
    }
    if m<=1 { return (0.0, 0.0); }
    let mf=m as f64; let cov=sxf - sx*sf/mf; let var=sff - sf*sf/mf; let varx=sxx - sx*sx/mf;
    if var<=0.0 || varx<=0.0 { return (0.0, 0.0); }
    let beta = cov/var;
    let r2 = (cov*cov)/(var*varx);
    (finite(beta), finite(r2))
}

/* ---------- handler ---------- */

pub async fn get_relative_strength(
    State(pool): State<PgPool>,
    Query(q): Query<RelativeStrengthRequest>,
) -> Json<RelativeStrengthResponse> {
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    let days = parse_period_days(&q.period);
    let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();
    let base = q.base_coin.clone().unwrap_or_else(|| "BTC".to_string()).to_uppercase();

    // universe (compare_coins OR topN)
    let mut syms: std::collections::BTreeSet<String> = q
        .compare_coins
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.to_uppercase())
        .collect();

    if let Some(n) = q.top_n {
        if let Ok(top) = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.market_type, days as i32, n).await {
            syms.extend(top.into_iter().map(|(s,_,_)| s.to_uppercase()));
        }
    }
    syms.remove(&base); // base is not ranked against itself
    let mut all_syms: Vec<String> = syms.into_iter().collect();
    all_syms.sort_unstable();
    all_syms.insert(0, base.clone()); // ensure base first

    // resolve market ids and load
    let mut mids = Vec::new();
    let mut sym_mid = Vec::new();
    for s in &all_syms {
        if let Ok(mid) = super::resolve_market_id_with_data(&pool, &q.exchange, s, &q.market_type, since_unix).await {
            mids.push(mid);
            sym_mid.push((s.clone(), mid));
        }
    }
    let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix).await.unwrap_or_default();

    // build aligned close series
    #[derive(Clone)]
    struct Serie { sym: String, ts: Vec<i64>, close: Vec<f64> }
    let mut series: Vec<Serie> = Vec::new();
    for (sym, mid) in sym_mid {
        if let Some(hourly) = by_mid.get(&mid) {
            let s = resample_from_hourly(hourly, tf.period_secs());
            if s.len() < q.window + 6 { continue; }
            let ts = s.iter().map(|r| r.ts).collect::<Vec<_>>();
            let close = s.iter().map(|r| r.close).collect::<Vec<_>>();
            series.push(Serie { sym, ts, close });
        }
    }
    if series.len() < 2 {
        return Json(RelativeStrengthResponse {
            base,
            rs_rankings: vec![],
            persistence: vec![],
            momentum_factor_loadings: vec![],
            rs_series: vec![],
            pair_divergence: vec![],
        });
    }

    // align on common timestamps
    let mut common = series[0].ts.clone();
    for s in &series[1..] {
        let mut inter = Vec::new();
        let (mut i,mut j)=(0,0);
        while i<common.len() && j<s.ts.len() {
            if common[i]==s.ts[j] { inter.push(common[i]); i+=1; j+=1; }
            else if common[i]<s.ts[j] { i+=1; } else { j+=1; }
        }
        common = inter;
    }
    if common.len() < q.window + 6 {
        return Json(RelativeStrengthResponse {
            base,
            rs_rankings: vec![],
            persistence: vec![],
            momentum_factor_loadings: vec![],
            rs_series: vec![],
            pair_divergence: vec![],
        });
    }

    // map symbol -> aligned closes
    let mut aligned: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for s in &series {
        let mut v = Vec::with_capacity(common.len());
        let mut i=0usize; let mut j=0usize;
        while i<s.ts.len() && j<common.len() {
            if s.ts[i]==common[j] { v.push(s.close[i]); j+=1; }
            i+=1;
        }
        if v.len()==common.len() { aligned.insert(s.sym.clone(), v); }
    }

    // compute RS z for each symbol vs base
    let base_close = aligned.get(&base).cloned().unwrap();
    let base_ln: Vec<f64> = base_close.iter().map(|p| p.ln()).collect();

    let mut rs_series = Vec::new();
    for (sym, close) in &aligned {
        if *sym == base { continue; }
        let ln: Vec<f64> = close.iter().map(|p| p.ln()).collect();
        let rel: Vec<f64> = ln.iter().zip(base_ln.iter()).map(|(a,b)| finite(a-b)).collect();
        let z = zscore_series(&rel, q.window);
        // only keep mature part
        let start = z.iter().position(|v| v.is_finite()).unwrap_or(z.len());
        if start >= z.len() { continue; }
        rs_series.push(RsSeries {
            symbol: sym.clone(),
            ts: common[start..].to_vec(),
            z:  z[start..].iter().map(|&v| finite(v)).collect(),
        });
    }
    if rs_series.is_empty() {
        return Json(RelativeStrengthResponse {
            base,
            rs_rankings: vec![],
            persistence: vec![],
            momentum_factor_loadings: vec![],
            rs_series: vec![],
            pair_divergence: vec![],
        });
    }

    // align RS series to same length (min)
    let min_len = rs_series.iter().map(|s| s.z.len()).min().unwrap_or(0);
    for s in &mut rs_series {
        if s.z.len() > min_len {
            let off = s.z.len() - min_len;
            s.z.drain(0..off);
            s.ts.drain(0..off);
        }
    }

    // rankings by last z
    let mut ranks: Vec<RsRank> = rs_series.iter()
        .map(|s| RsRank { pair: format!("{}/{}", s.symbol, base), last_z: s.z[min_len-1] })
        .collect();
    ranks.sort_by(|a,b| b.last_z.total_cmp(&a.last_z));

    // persistence (lag-1 autocorr of RS z)
    let persistence: Vec<PersistenceRow> = rs_series.iter().map(|s| {
        let (x,y) = (&s.z[1..], &s.z[..s.z.len()-1]);
        PersistenceRow { symbol: s.symbol.clone(), rho1: pearson(x,y) }
    }).collect();

    // momentum factor loading: Δz vs common Δz factor
    let dz_all: Vec<Vec<f64>> = rs_series.iter().map(|s| {
        s.z.windows(2).map(|w| finite(w[1]-w[0])).collect::<Vec<_>>()
    }).collect();
    let steps = dz_all[0].len();
    let mut factor = vec![0.0; steps];
    for t in 0..steps {
        let mut sum=0.0; let mut c=0.0;
        for j in 0..dz_all.len() { let v = dz_all[j][t]; if v.is_finite() { sum+=v; c+=1.0; } }
        factor[t] = if c>0.0 { sum/c } else { 0.0 };
    }
    let loadings: Vec<FactorLoading> = rs_series.iter().enumerate().map(|(i,s)| {
        let (beta, r2) = beta_r2(&dz_all[i], &factor);
        FactorLoading { symbol: s.symbol.clone(), beta, r2 }
    }).collect();

    // pair divergence: top 5 |Δz| between any two series at each t (report time series for top by last |div|)
    let mut pair_divs: Vec<(String, Vec<DivPoint>, f64)> = Vec::new();
    for i in 0..rs_series.len() {
        for j in (i+1)..rs_series.len() {
            let (si, sj) = (&rs_series[i], &rs_series[j]);
            let mut rows = Vec::with_capacity(min_len);
            for k in 0..min_len {
                let d = finite(si.z[k] - sj.z[k]);
                rows.push(DivPoint { timestamp: si.ts[k], z1: si.z[k], z2: sj.z[k], divergence: d });
            }
            let last_abs = rows.last().map(|r| r.divergence.abs()).unwrap_or(0.0);
            pair_divs.push((format!("{}-{}", si.symbol, sj.symbol), rows, last_abs));
        }
    }
    pair_divs.sort_by(|a,b| b.2.total_cmp(&a.2));
    let pair_divergence = pair_divs.into_iter().take(5).map(|(p, rows, _)| PairDiv { pair: p, time_series: rows }).collect();

    Json(RelativeStrengthResponse {
        base,
        rs_rankings: ranks,
        persistence,
        momentum_factor_loadings: loadings,
        rs_series,
        pair_divergence,
    })
}
