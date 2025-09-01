use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;

use super::{
    parse_period_days, top_markets_by_usd_volume_live, Tf,
    resolve_market_id_with_data,
};

fn default_market_type() -> String { "spot".to_string() }
fn default_timeframe() -> String { "1h".to_string() }
fn default_period() -> String { "7d".to_string() }
#[inline] fn finite(x: f64) -> f64 { if x.is_finite() { x } else { 0.0 } }

#[derive(Debug, Deserialize)]
pub struct MicrostructureRequest {
    #[serde(default = "default_timeframe")]
    pub timeframe: String,            // "1h" ONLY

    #[serde(default)]
    pub topN: Option<i64>,            // default 50

    pub exchange: String,
    #[serde(default = "default_market_type")]
    pub marketType: String,

    #[serde(default = "default_period")]
    pub period: String,               // e.g. "7d"
}

#[derive(Debug, Serialize)]
pub struct MicrostructureResponse {
    pub volumeFlows: Vec<FlowRow>,
    pub rotationMatrix: RotationMatrix,
    pub liquidityConcentration: LiquidityConcentration,
}

#[derive(Debug, Serialize)]
pub struct FlowRow {
    pub symbol: String,
    pub volumeIn: f64,     // buy USD total
    pub volumeOut: f64,    // sell USD total
    pub netFlow: f64,      // buy - sell
    pub netFlowZScore: f64,
}

#[derive(Debug, Serialize)]
pub struct RotationMatrix {
    pub coins: Vec<String>,
    pub flows: Vec<Vec<f64>>,   // i -> j
}

#[derive(Debug, Serialize)]
pub struct LiquidityConcentration {
    pub groups: Vec<LiqGroup>,
}

#[derive(Debug, Serialize)]
pub struct LiqGroup {
    pub range: String,      // "Top 5", "6-10", ...
    pub volumeShare: f64,
    pub countShare: f64,
}

pub async fn get_microstructure_flow(
    State(pool): State<PgPool>,
    Query(q): Query<MicrostructureRequest>,
) -> Json<MicrostructureResponse> {
    // enforce 1h
    let tf = Tf::from_str(&q.timeframe).unwrap_or(Tf::H1);
    if !matches!(tf, Tf::H1) {
        return Json(MicrostructureResponse {
            volumeFlows: vec![],
            rotationMatrix: RotationMatrix { coins: vec![], flows: vec![] },
            liquidityConcentration: LiquidityConcentration { groups: vec![] },
        });
    }

    let days = parse_period_days(&q.period);
    let since_unix = (time::OffsetDateTime::now_utc() - time::Duration::days(days)).unix_timestamp();
    let n = q.topN.unwrap_or(50);

    // universe
    let mut universe = top_markets_by_usd_volume_live(&pool, &q.exchange, &q.marketType, days as i32, n)
        .await
        .unwrap_or_default();

    if universe.is_empty() {
        return Json(MicrostructureResponse {
            volumeFlows: vec![],
            rotationMatrix: RotationMatrix { coins: vec![], flows: vec![] },
            liquidityConcentration: LiquidityConcentration { groups: vec![] },
        });
    }

    // Ensure market ids exist (some helper expects data; fall back to resolver)
    // The original code tried to dereference `mid` which was a value, not a reference.
    // By destructuring `&mut universe`, we get mutable references to the tuple's elements,
    // allowing us to correctly dereference and update the `mid` value.
    for (sym, mid, _) in &mut universe {
        if *mid == 0 {
            if let Ok(new_mid) = resolve_market_id_with_data(&pool, &q.exchange, sym, &q.marketType, since_unix).await {
                *mid = new_mid;
            }
        }
    }


    // Pull hourly aggregated trades per coin
    // We'll query per market_id to avoid ARRAY param hassles
    #[derive(sqlx::FromRow, Debug)]
    struct AggRow { ts: time::OffsetDateTime, buy_usd: Option<f64>, sell_usd: Option<f64> }

    let mut hourly: HashMap<String, Vec<(i64, f64, f64)>> = HashMap::new();
    for (sym, mid, _) in &universe {
        let recs: Vec<AggRow> = sqlx::query_as::<_, AggRow>(
            r#"
            SELECT date_trunc('hour', trade_time) AS ts,
                   SUM(CASE WHEN side='buy'  THEN price*qty ELSE 0 END) AS buy_usd,
                   SUM(CASE WHEN side='sell' THEN price*qty ELSE 0 END) AS sell_usd
            FROM trades
            WHERE market_id = $1 AND trade_time >= to_timestamp($2)
            GROUP BY 1
            ORDER BY 1
            "#
        )
        .bind(*mid)
        .bind(since_unix)
        .fetch_all(&pool).await.unwrap_or_default();

        let v = recs.into_iter().map(|r| {
            (r.ts.unix_timestamp(), r.buy_usd.unwrap_or(0.0), r.sell_usd.unwrap_or(0.0))
        }).collect::<Vec<_>>();
        hourly.insert(sym.clone(), v);
    }

    // Totals & net zscores
    let mut totals: Vec<(String, f64, f64, f64)> = Vec::new(); // sym, buy, sell, net
    for (sym, series) in &hourly {
        let mut b = 0.0; let mut s = 0.0;
        for &(_, bu, se) in series { b += bu; s += se; }
        totals.push((sym.clone(), b, s, b - s));
    }

    // z-score of netFlow across universe
    let mut nets: Vec<f64> = totals.iter().map(|t| t.3).collect();
    let mean = if nets.is_empty() { 0.0 } else { nets.iter().sum::<f64>() / (nets.len() as f64) };
    let var = if nets.is_empty() { 0.0 } else {
        let mut ss = 0.0; for &x in &nets { ss += (x - mean)*(x - mean); } ss / (nets.len() as f64)
    };
    let sd = var.max(0.0).sqrt();

    let mut volume_flows: Vec<FlowRow> = totals.into_iter().map(|(sym, b, s, n)| {
        FlowRow {
            symbol: sym, volumeIn: finite(b), volumeOut: finite(s),
            netFlow: finite(n),
            netFlowZScore: if sd>0.0 { (n - mean)/sd } else { 0.0 },
        }
    }).collect();

    // Order by absolute net flow
    volume_flows.sort_by(|a,b| b.netFlow.abs().total_cmp(&a.netFlow.abs()));

    // Rotation matrix via hourly allocation P->N
    // Build coin index list (limit to 20 for a compact matrix)
    let mut coins: Vec<String> = hourly.keys().cloned().collect();
    coins.sort();
    let keep = coins.len().min(20);
    coins.truncate(keep);

    let mut idx: HashMap<String, usize> = HashMap::new();
    for (i, s) in coins.iter().enumerate() { idx.insert(s.clone(), i); }

    // Create per-hour net per coin
    // First, gather all hours union
    use std::collections::BTreeSet;
    let mut all_hours: BTreeSet<i64> = BTreeSet::new();
    for s in &coins {
        if let Some(v) = hourly.get(s) {
            for &(t,_,_) in v { all_hours.insert(t); }
        }
    }

    let mut flow = vec![vec![0.0; keep]; keep]; // i -> j
    for t in all_hours {
        let mut pos: Vec<(usize, f64)> = Vec::new();
        let mut neg: Vec<(usize, f64)> = Vec::new();
        let mut sum_pos = 0.0;
        let mut sum_abs_neg = 0.0;

        for s in &coins {
            let i = idx[s];
            let mut net = 0.0;
            if let Some(v) = hourly.get(s) {
                if let Some(&(_, bu, se)) = v.iter().find(|(ts,_,_)| *ts == t) {
                    net = bu - se;
                }
            }
            if net > 0.0 { sum_pos += net; pos.push((i, net)); }
            else if net < 0.0 { sum_abs_neg += -net; neg.push((i, -net)); }
        }

        if sum_pos > 0.0 && sum_abs_neg > 0.0 {
            for &(i, p) in &pos {
                for &(j, nabs) in &neg {
                    // allocate p across negatives prop to their size
                    let amt = p * (nabs / sum_abs_neg);
                    flow[i][j] += finite(amt);
                }
            }
        }
    }

    // Liquidity concentration from total USD volume (buy+sell)
    let mut vol_pairs: Vec<(String, f64)> = hourly.iter().map(|(s, v)| {
        let mut sum = 0.0; for &(_, bu, se) in v { sum += bu + se; } (s.clone(), sum)
    }).collect();
    vol_pairs.sort_by(|a,b| b.1.total_cmp(&a.1));
    let total_vol: f64 = vol_pairs.iter().map(|p| p.1).sum();

    let mut groups = Vec::new();
    let ranges = vec![(1,5,"Top 5"), (6,10,"6-10"), (11,20,"11-20")];
    let n_coins = vol_pairs.len() as f64;

    for (a,b,label) in ranges {
        if vol_pairs.is_empty() { continue; }
        let start = (a-1).min(vol_pairs.len());
        let end = b.min(vol_pairs.len());
        if start >= end { continue; }
        let slice = &vol_pairs[start..end];
        let share = if total_vol>0.0 { slice.iter().map(|x| x.1).sum::<f64>() / total_vol } else { 0.0 };
        groups.push(LiqGroup {
            range: label.to_string(),
            volumeShare: finite(share),
            countShare: finite((slice.len() as f64) / n_coins),
        });
    }
    // remaining
    if vol_pairs.len() > 20 {
        let slice = &vol_pairs[20..];
        let share = if total_vol>0.0 { slice.iter().map(|x| x.1).sum::<f64>() / total_vol } else { 0.0 };
        groups.push(LiqGroup {
            range: "21+".to_string(),
            volumeShare: finite(share),
            countShare: finite((slice.len() as f64) / n_coins),
        });
    }

    Json(MicrostructureResponse {
        volumeFlows: volume_flows,
        rotationMatrix: RotationMatrix { coins, flows: flow },
        liquidityConcentration: LiquidityConcentration { groups },
    })
}
