

use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{Error as DeError, SeqAccess, Visitor};
use sqlx::PgPool;
use std::{fmt, collections::BTreeMap};

use super::{fetch_multi_hourly_ohlcv, parse_period_days, resample_from_hourly, top_markets_by_usd_volume_live, zscore_series, Tf};

fn default_market_type() -> String { "spot".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
fn default_window() -> usize { 48 }
fn default_vel_win() -> usize { 6 }
#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }

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
    /// timeframes like ["1h","4h","1d"] (we’ll map 3d/7d to multiples of 1d)
    #[serde(default = "default_tfs", deserialize_with = "string_or_seq")]
    pub timeframes: Vec<String>,
}
fn default_tfs() -> Vec<String> { vec!["1h".into(),"4h".into(),"1d".into()] }

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegimeMomentumResponse {
    pub heatmap: Heatmap,                // coins x timeframes (Δz over velocity_window)
    pub transition_matrix: [[f64; 3]; 3],// bull/bear/range transition probs (row norm)
    pub velocity_series: Vec<VelSeries>, // per coin time series of Δz
    pub cross_asset_divergence: DivergenceSnapshot,
}
#[derive(Debug, Serialize)]
pub struct Heatmap { pub coins: Vec<String>, pub timeframes: Vec<String>, pub matrix: Vec<Vec<f64>> }
#[derive(Debug, Serialize)]
pub struct VelSeries { pub symbol: String, pub ts: Vec<i64>, pub dz: Vec<f64> }
#[derive(Debug, Serialize)]
pub struct DivergenceSnapshot { pub leaders: Vec<String>, pub laggards: Vec<String>, pub score: f64 }


fn to_tf(s: &str) -> Tf {
    match Tf::from_str(s) {
        Some(tf) => tf,
        None => {
            if s.eq_ignore_ascii_case("3d") { Tf::D1 }
            else if s.eq_ignore_ascii_case("7d") { Tf::D1 }
            else { Tf::H1 }
        }
    }
}

fn state_from_z(z: f64) -> usize {
    // 0 bull, 1 bear, 2 range
    if z > 0.5 { 0 } else if z < -0.5 { 1 } else { 2 }
}

pub async fn get_regime_momentum(
    State(pool): State<PgPool>,
    Query(q): Query<RegimeMomentumRequest>,
) -> Json<RegimeMomentumResponse> {

    let days = parse_period_days(&q.period);
    let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();

    // universe
    let mut syms: std::collections::BTreeSet<String> =
        q.coins.clone().unwrap_or_default().into_iter().map(|s| s.to_uppercase()).collect();
    if let Some(n) = q.top_n {
        if let Ok(top) = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.market_type, days as i32, n).await {
            syms.extend(top.into_iter().map(|(s,_,_)| s.to_uppercase()));
        }
    }
    let mut all_syms: Vec<String> = syms.into_iter().collect();
    all_syms.sort_unstable();

    // resolve + fetch once (we’ll resample for each TF)
    let mut mids = Vec::new(); let mut sym_mid = Vec::new();
    for s in &all_syms {
        if let Ok(mid) = super::resolve_market_id_with_data(&pool, &q.exchange, s, &q.market_type, since_unix).await {
            mids.push(mid); sym_mid.push((s.clone(), mid));
        }
    }
    let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix).await.unwrap_or_default();
    if sym_mid.is_empty() {
        return Json(RegimeMomentumResponse {
            heatmap: Heatmap { coins: vec![], timeframes: vec![], matrix: vec![] },
            transition_matrix: [[0.0;3];3],
            velocity_series: vec![],
            cross_asset_divergence: DivergenceSnapshot { leaders: vec![], laggards: vec![], score: 0.0 },
        });
    }

    // build, for base TF (1h), aligned z and dz for transition/divergence
    let tf_base = Tf::H1;
    #[derive(Clone)] struct ZS { sym: String, ts: Vec<i64>, z: Vec<f64> }
    let mut zsers: Vec<ZS> = Vec::new();
    for (sym, mid) in &sym_mid {
        if let Some(h) = by_mid.get(mid) {
            let s = resample_from_hourly(h, tf_base.period_secs());
            if s.len() < q.window + 6 { continue; }
            let ts = s.iter().map(|r| r.ts).collect::<Vec<_>>();
            let close = s.iter().map(|r| r.close).collect::<Vec<_>>();
            let z = zscore_series(&close, q.window);
            let start = z.iter().position(|v| v.is_finite()).unwrap_or(z.len());
            if start >= z.len() { continue; }
            zsers.push(ZS { sym: sym.clone(), ts: ts[start..].to_vec(), z: z[start..].iter().map(|&v| finite(v)).collect() });
        }
    }
    if zsers.is_empty() {
        return Json(RegimeMomentumResponse {
            heatmap: Heatmap { coins: vec![], timeframes: q.timeframes.clone(), matrix: vec![] },
            transition_matrix: [[0.0;3];3],
            velocity_series: vec![],
            cross_asset_divergence: DivergenceSnapshot { leaders: vec![], laggards: vec![], score: 0.0 },
        });
    }
    let min_len = zsers.iter().map(|s| s.z.len()).min().unwrap();
    for s in &mut zsers {
        if s.z.len()>min_len { let off=s.z.len()-min_len; s.z.drain(0..off); s.ts.drain(0..off); }
    }

    // velocity series (Δz per step)
    let mut vel_series = Vec::new();
    for s in &zsers {
        let dz: Vec<f64> = s.z.windows(2).map(|w| finite(w[1]-w[0])).collect();
        vel_series.push(VelSeries { symbol: s.sym.clone(), ts: s.ts[1..].to_vec(), dz });
    }

    // regime transition on base TF using the cross-sectional mean z
    let mut mean_z = vec![0.0; min_len];
    for t in 0..min_len {
        let mut sum=0.0; let mut c=0.0;
        for s in &zsers { let v=s.z[t]; if v.is_finite(){ sum+=v; c+=1.0; } }
        mean_z[t] = if c>0.0 { sum/c } else { 0.0 };
    }
    let mut counts = [[0usize;3];3];
    for t in 1..min_len {
        let a = state_from_z(mean_z[t-1]);
        let b = state_from_z(mean_z[t]);
        counts[a][b] += 1;
    }
    let mut trans = [[0.0;3];3];
    for i in 0..3 {
        let row_sum: usize = counts[i].iter().sum();
        if row_sum>0 {
            for j in 0..3 { trans[i][j] = counts[i][j] as f64 / row_sum as f64; }
        }
    }

    // heatmap across requested timeframes: value = last Δz over velocity_window on each TF
    let mut heat_matrix: Vec<Vec<f64>> = Vec::new();
    for (sym, mid) in &sym_mid {
        let mut row = Vec::with_capacity(q.timeframes.len());
        'tfloop: for tf_str in &q.timeframes {
            let tf = to_tf(tf_str);
            let Some(h) = by_mid.get(mid) else { row.push(0.0); continue; };
            let s = resample_from_hourly(h, tf.period_secs());
            if s.len() < q.window + q.velocity_window + 6 { row.push(0.0); continue 'tfloop; }
            let close = s.iter().map(|r| r.close).collect::<Vec<_>>();
            let z = zscore_series(&close, q.window);
            let start = z.iter().position(|v| v.is_finite()).unwrap_or(z.len());
            if start + q.velocity_window >= z.len() { row.push(0.0); continue 'tfloop; }
            let last = z.len()-1;
            row.push( finite(z[last] - z[last - q.velocity_window]) );
        }
        heat_matrix.push(row);
    }
    // order rows to match zsers (alphabetical by coins)
    let coin_order: Vec<String> = sym_mid.iter().map(|(s,_)| s.clone()).collect();

    // leaders/laggards & divergence score from base TF velocity (last Δz)
    let mut last_dz: Vec<(String,f64)> = vel_series.iter()
        .map(|v| (v.symbol.clone(), v.dz.last().copied().unwrap_or(0.0))).collect();
    last_dz.sort_by(|a,b| b.1.total_cmp(&a.1));
    let k = (last_dz.len().max(10) as f64 * 0.2) as usize; // top/bottom 20%
    let leaders = last_dz.iter().take(k).map(|x| x.0.clone()).collect::<Vec<_>>();
    let laggards = last_dz.iter().rev().take(k).map(|x| x.0.clone()).collect::<Vec<_>>();
    let mean_top = if k>0 { leaders.iter().map(|s| last_dz.iter().find(|x| &x.0==s).unwrap().1).sum::<f64>()/k as f64 } else {0.0};
    let mean_bot = if k>0 { laggards.iter().map(|s| last_dz.iter().find(|x| &x.0==s).unwrap().1).sum::<f64>()/k as f64 } else {0.0};
    // dispersion (std of last dz)
    let mu = last_dz.iter().map(|x| x.1).sum::<f64>() / (last_dz.len() as f64).max(1.0);
    let var = last_dz.iter().map(|x| { let d=x.1-mu; d*d }).sum::<f64>() / (last_dz.len() as f64).max(1.0);
    let sigma = var.sqrt();
    let div_score = if sigma>0.0 { (mean_top - mean_bot)/sigma } else { 0.0 };

    Json(RegimeMomentumResponse {
        heatmap: Heatmap { coins: coin_order, timeframes: q.timeframes.clone(), matrix: heat_matrix },
        transition_matrix: trans,
        velocity_series: vel_series,
        cross_asset_divergence: DivergenceSnapshot { leaders, laggards, score: finite(div_score) },
    })
}
