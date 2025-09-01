use axum::{extract::{Query, State}, Json};
use serde::{Deserialize,Deserializer, Serialize};
use sqlx::PgPool;

use super::{
    de_string_or_vec, fetch_multi_hourly_ohlcv, parse_period_days, resample_from_hourly,
    top_markets_by_usd_volume_live, zscore_series, Tf,
};


use serde::de::{Error as DeError, SeqAccess, Visitor};
use std::fmt;

// Accept string, CSV string, or a sequence (?coins[]=A&coins[]=B, or repeated coins=)
fn string_or_seq<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StrOrSeq;
    impl<'de> Visitor<'de> for StrOrSeq {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "string, CSV string, or sequence of strings")
        }
        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where E: DeError {
            let parts: Vec<String> = v
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            Ok(if parts.is_empty() { vec![v.to_string()] } else { parts })
        }
        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where E: DeError {
            self.visit_str(&v)
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: SeqAccess<'de> {
            let mut out = Vec::new();
            while let Some(elem) = seq.next_element::<String>()? {
                if elem.contains(',') {
                    out.extend(
                        elem.split(',')
                           .map(|s| s.trim())
                           .filter(|s| !s.is_empty())
                           .map(|s| s.to_string()),
                    );
                } else {
                    out.push(elem);
                }
            }
            Ok(out)
        }
    }
    de.deserialize_any(StrOrSeq)
}


fn default_market_type() -> String { "spot".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }




#[derive(Debug, Deserialize)]
pub struct InterAssetZScoreRequest {
    #[serde(default, deserialize_with = "string_or_seq")]
    pub coins: Vec<String>,
    pub period: String,
    pub exchange: String,
    #[serde(default = "default_market_type")]
    pub marketType: String,
    #[serde(default)]
    pub topN: Option<i64>,
    #[serde(default = "default_window")]
    pub window: usize,
    #[serde(default = "default_timeframe")]
    pub timeframe: String,
    #[serde(default)]
    pub indexCoin: Option<String>,
}


fn default_window() -> usize { 48 } // 2 days @1h

#[derive(Debug, Serialize)]
pub struct InterAssetZScoreResponse {
    pub zscoreCorrelationMatrix: ZCorrMat,
    pub zscoreBetaMatrix: ZBetaMat,
    pub pairDivergence: Vec<PairDiv>,
}

#[derive(Debug, Serialize)]
pub struct ZCorrMat {
    pub coins: Vec<String>,
    pub matrix: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct ZBetaMat {
    pub coins: Vec<String>,  // same order as corr coins
    pub matrix: Vec<Vec<f64>>, // for completeness, diagonal 1.0 for index row
}

#[derive(Debug, Serialize)]
pub struct PairDiv {
    pub pair: String, // "BTC-ETH"
    pub timeSeries: Vec<DivergenceRow>,
}

#[derive(Debug, Serialize)]
pub struct DivergenceRow {
    pub timestamp: i64,
    pub zscore1: f64,
    pub zscore2: f64,
    pub divergence: f64, // z1 - z2
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len().min(y.len()); if n==0 {return 0.0;}
    let (mut sx,mut sy,mut sxx,mut syy,mut sxy)=(0.0,0.0,0.0,0.0,0.0); let mut m=0usize;
    for i in 0..n { let (a,b)=(x[i],y[i]); if a.is_finite()&&b.is_finite(){sx+=a;sy+=b;sxx+=a*a;syy+=b*b;sxy+=a*b;m+=1;} }
    if m<=1 {return 0.0;} let mf=m as f64; let cov=sxy - sx*sy/mf; let vx=sxx - sx*sx/mf; let vy=syy - sy*sy/mf;
    if vx<=0.0||vy<=0.0 {0.0} else { finite(cov/(vx.sqrt()*vy.sqrt())) }
}

fn beta_vs(x: &[f64], idx: &[f64]) -> f64 {
    let n = x.len().min(idx.len()); if n==0 {return 0.0;}
    let (mut sx,mut si,mut sxx,mut sii,mut sxi)=(0.0,0.0,0.0,0.0,0.0); let mut m=0usize;
    for i in 0..n { let (a,b)=(x[i],idx[i]); if a.is_finite()&&b.is_finite(){sx+=a;si+=b;sxx+=a*a;sii+=b*b;sxi+=a*b;m+=1;} }
    if m<=1 {return 0.0;} let mf=m as f64; let cov=sxi - sx*si/mf; let var=sii - si*si/mf;
    if var<=0.0 {0.0} else { finite(cov/var) }
}

pub async fn get_inter_asset_zscore(
    State(pool): State<PgPool>,
    Query(q): Query<InterAssetZScoreRequest>,
) -> Json<InterAssetZScoreResponse> {
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    let days = parse_period_days(&q.period);
    let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();

    // Universe
    let mut symbols: Vec<String> = if !q.coins.is_empty() {
        q.coins.iter().map(|s| s.to_uppercase()).collect()
    } else {
        top_markets_by_usd_volume_live(&pool, &q.exchange, &q.marketType, days as i32, q.topN.unwrap_or(30))
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(s,_,_)| s)
            .collect()
    };
    if symbols.is_empty() {
        return Json(InterAssetZScoreResponse {
            zscoreCorrelationMatrix: ZCorrMat { coins: vec![], matrix: vec![] },
            zscoreBetaMatrix: ZBetaMat { coins: vec![], matrix: vec![] },
            pairDivergence: vec![],
        });
    }

    // Resolve market ids and load hourly data
    let mut mids: Vec<i32> = Vec::new();
    let mut sym_mid: Vec<(String, i32)> = Vec::new();
    for s in &symbols {
        if let Ok(mid) = super::resolve_market_id_with_data(&pool, &q.exchange, s, &q.marketType, since_unix).await {
            mids.push(mid);
            sym_mid.push((s.clone(), mid));
        }
    }
    let by_mid = fetch_multi_hourly_ohlcv(&pool, &mids, since_unix).await.unwrap_or_default();

    // Build aligned z-score series
    #[derive(Clone)]
    struct ZSer { sym: String, ts: Vec<i64>, z: Vec<f64> }
    let mut zsers: Vec<ZSer> = Vec::new();

    for (sym, mid) in sym_mid {
        let Some(hourly) = by_mid.get(&mid) else { continue; };
        let ser = resample_from_hourly(hourly, tf.period_secs());
        if ser.len() < q.window + 6 { continue; }
        let ts: Vec<i64> = ser.iter().map(|r| r.ts).collect();
        let close: Vec<f64> = ser.iter().map(|r| r.close).collect();

        let z = zscore_series(&close, q.window);
        let start = z.iter().position(|v| v.is_finite()).unwrap_or(z.len());
        if start >= z.len() { continue; }

        zsers.push(ZSer {
            sym,
            ts: ts[start..].to_vec(),
            z:  z[start..].iter().map(|&v| finite(v)).collect(),
        });
    }

    if zsers.len() < 2 {
        return Json(InterAssetZScoreResponse {
            zscoreCorrelationMatrix: ZCorrMat { coins: vec![], matrix: vec![] },
            zscoreBetaMatrix: ZBetaMat { coins: vec![], matrix: vec![] },
            pairDivergence: vec![],
        });
    }

    // Align all by last common length
    let min_len = zsers.iter().map(|s| s.z.len()).min().unwrap_or(0);
    for s in &mut zsers {
        if s.z.len() > min_len {
            let off = s.z.len() - min_len;
            s.z.drain(0..off);
            s.ts.drain(0..off);
        }
    }

    let coins: Vec<String> = zsers.iter().map(|s| s.sym.clone()).collect();
    let m = zsers.len();

    // Choose index
    let idx_name = q.indexCoin.clone().unwrap_or_else(|| "BTC".to_string()).to_uppercase();
    let mut idx_pos = 0usize;
    if let Some(pos) = coins.iter().position(|s| s == &idx_name) { idx_pos = pos; }
    if idx_pos != 0 {
        zsers.swap(0, idx_pos);
    }

    // Correlation of z-scores
    let mut corr = vec![vec![0.0; m]; m];
    for i in 0..m {
        for j in i..m {
            let c = pearson(&zsers[i].z, &zsers[j].z);
            corr[i][j] = c; corr[j][i] = c;
        }
    }

    // Beta of z-scores vs index
    let idx = &zsers[0].z;
    let mut beta = vec![vec![0.0; 1]; m];
    for i in 0..m {
        beta[i][0] = beta_vs(&zsers[i].z, idx);
    }

    // Pair divergence: pick top 5 by |last divergence|
    let mut pair_divs: Vec<(String, Vec<DivergenceRow>, f64)> = Vec::new();
    for i in 0..m {
        for j in (i+1)..m {
            let z1 = &zsers[i].z;
            let z2 = &zsers[j].z;
            let ts = &zsers[i].ts;
            let n = z1.len().min(z2.len());
            if n == 0 { continue; }
            let mut rows = Vec::with_capacity(n);
            for k in 0..n {
                let d = finite(z1[k] - z2[k]);
                rows.push(DivergenceRow {
                    timestamp: ts[k],
                    zscore1: z1[k],
                    zscore2: z2[k],
                    divergence: d,
                });
            }
            let last_abs = rows.last().map(|r| r.divergence.abs()).unwrap_or(0.0);
            pair_divs.push((format!("{}-{}", zsers[i].sym, zsers[j].sym), rows, last_abs));
        }
    }
    pair_divs.sort_by(|a,b| b.2.total_cmp(&a.2));
    let pairDivergence = pair_divs.into_iter().take(5).map(|(p, rows, _)| PairDiv {
        pair: p, timeSeries: rows,
    }).collect();

    Json(InterAssetZScoreResponse {
        zscoreCorrelationMatrix: ZCorrMat { coins, matrix: corr },
        zscoreBetaMatrix: ZBetaMat { coins: zsers.iter().map(|s| s.sym.clone()).collect(), matrix: beta },
        pairDivergence,
    })
}


