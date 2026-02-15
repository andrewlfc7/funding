use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/* ---------- query params ---------- */

#[derive(Debug, Deserialize)]
pub struct VolFcstParams {
    pub exchange: String,
    pub market_type: String, // "spot" | "perps"
    #[serde(alias = "base", alias = "ticker")]
    pub symbol: String, // e.g. "BTC"
    pub days: Option<i64>,   // default 180
    pub short_span: Option<usize>, // default 30
    pub long_span: Option<usize>, // default 120
    pub blend_weight: Option<f64>, // default 0.5 in [0,1]
    pub annualize: Option<bool>, // default false
    pub log_returns: Option<bool>, // default true (else simple returns)
}

/* ---------- output ---------- */

#[derive(Debug, Serialize)]
pub struct VolFcstPoint {
    pub ts: i64,        // epoch millis (close timestamp)
    pub r2: f64,        // squared daily return aligned to ts (from t-1 -> t)
    pub vol_short: f64, // sqrt(EMA_s(r^2)[t-1]) * ann?
    pub vol_long: f64,  // sqrt(EMA_l(r^2)[t-1]) * ann?
    pub vol_blend: f64, // sqrt(w*v_s + (1-w)*v_l)[t-1] * ann?
}

/* ---------- helpers ---------- */

fn parse_market_type(s: &str) -> Result<&'static str, String> {
    match s {
        "spot" => Ok("spot"),
        "perps" => Ok("perps"),
        other => Err(format!("invalid market_type '{}'", other)),
    }
}

#[derive(sqlx::FromRow)]
struct KRow {
    ts: i64,
    close: f64,
}

async fn fetch_daily_close(
    pool: &PgPool,
    exchange: &str,
    market_type: &str,
    base: &str,
    days: i64,
) -> Result<Vec<KRow>, (axum::http::StatusCode, String)> {
    let mt =
        parse_market_type(market_type).map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

    let rows = sqlx::query_as!(
        KRow,
        r#"
        WITH preferred AS (
          SELECT m.id AS market_id
          FROM cex_markets m
          JOIN cex_exchanges e ON e.id = m.exchange_id
          WHERE e.name = $1
            AND m.market_type = $2
            AND UPPER(m.market_symbol) IN (
              UPPER($3) || 'USDT', UPPER($3) || 'USDC', UPPER($3) || 'FDUSD',
              UPPER($3) || 'BUSD', UPPER($3) || 'TUSD', UPPER($3) || 'USD',
              UPPER($3) || 'DAI',  UPPER($3) || 'USDP'
            )
          ORDER BY CASE
            WHEN UPPER(m.market_symbol) = UPPER($3) || 'USDT' THEN 1
            WHEN UPPER(m.market_symbol) = UPPER($3) || 'USDC' THEN 2
            WHEN UPPER(m.market_symbol) = UPPER($3) || 'FDUSD' THEN 3
            WHEN UPPER(m.market_symbol) = UPPER($3) || 'BUSD' THEN 4
            WHEN UPPER(m.market_symbol) = UPPER($3) || 'TUSD' THEN 5
            WHEN UPPER(m.market_symbol) = UPPER($3) || 'USD'  THEN 6
            WHEN UPPER(m.market_symbol) = UPPER($3) || 'DAI'  THEN 7
            WHEN UPPER(m.market_symbol) = UPPER($3) || 'USDP' THEN 8
            ELSE 99
          END
          LIMIT 1
        )
        SELECT
          (EXTRACT(EPOCH FROM k.date) * 1000)::bigint AS "ts!",
          k.close::float8 AS "close!"
        FROM klines_daily k
        JOIN preferred p ON p.market_id = k.market_id
        WHERE k.date >= NOW() - ($4 * INTERVAL '1 day')
        ORDER BY k.date ASC
        "#,
        exchange,
        mt,
        base,
        days as f64
    )
    .fetch_all(pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(rows)
}

fn ewma_alpha(values: &[f64], alpha: f64) -> Vec<f64> {
    if values.is_empty() {
        return vec![];
    }
    let mut out = Vec::with_capacity(values.len());
    let mut s = values[0];
    out.push(s);
    for &x in &values[1..] {
        s = alpha * x + (1.0 - alpha) * s;
        out.push(s);
    }
    out
}

/* ---------- endpoint ---------- */

pub async fn get_vol_fcst_ema(
    State(pool): State<PgPool>,
    Query(q): Query<VolFcstParams>,
) -> Result<Json<Vec<VolFcstPoint>>, (axum::http::StatusCode, String)> {
    let days = q.days.unwrap_or(180).max(3);
    let sspan = q.short_span.unwrap_or(30).max(2);
    let lspan = q.long_span.unwrap_or(120).max(sspan + 1);
    let w = q.blend_weight.unwrap_or(0.5).clamp(0.0, 1.0);
    let ann = q.annualize.unwrap_or(false);
    let use_log = q.log_returns.unwrap_or(true);

    let rows = fetch_daily_close(&pool, &q.exchange, &q.market_type, &q.symbol, days).await?;
    if rows.len() < 3 {
        return Ok(Json(vec![]));
    }

    // returns
    let mut rets = Vec::<f64>::with_capacity(rows.len());
    rets.push(0.0);
    for wdw in rows.windows(2) {
        let p0 = wdw[0].close;
        let p1 = wdw[1].close;
        let r = if use_log {
            if p0 > 0.0 && p1 > 0.0 {
                (p1 / p0).ln()
            } else {
                0.0
            }
        } else {
            if p0 > 0.0 { (p1 / p0) - 1.0 } else { 0.0 }
        };
        rets.push(r);
    }

    // r^2 series aligned to close timestamps
    let r2: Vec<f64> = rets.iter().map(|r| r * r).collect();

    // EMA of r^2 for short & long
    let a_s = 2.0 / (sspan as f64 + 1.0);
    let a_l = 2.0 / (lspan as f64 + 1.0);
    let v_s_raw = ewma_alpha(&r2, a_s);
    let v_l_raw = ewma_alpha(&r2, a_l);

    // shift by 1 day (forecast uses t-1 info)
    let mut v_s = vec![std::f64::NAN; v_s_raw.len()];
    let mut v_l = vec![std::f64::NAN; v_l_raw.len()];
    for i in 1..v_s_raw.len() {
        v_s[i] = v_s_raw[i - 1];
        v_l[i] = v_l_raw[i - 1];
    }

    // sqrt + optional annualization
    let ann_factor = if ann { (365.0_f64).sqrt() } else { 1.0 };
    let eps = 1e-12;

    let mut out = Vec::<VolFcstPoint>::with_capacity(rows.len());
    for i in 0..rows.len() {
        let vs = (v_s[i].max(0.0) + eps).sqrt() * ann_factor;
        let vl = (v_l[i].max(0.0) + eps).sqrt() * ann_factor;
        let vb = ((w * v_s[i].max(0.0) + (1.0 - w) * v_l[i].max(0.0)) + eps).sqrt() * ann_factor;
        out.push(VolFcstPoint {
            ts: rows[i].ts,
            r2: r2[i],
            vol_short: vs,
            vol_long: vl,
            vol_blend: vb,
        });
    }

    // drop leading NaNs (first forecast point)
    while let Some(first) = out.first() {
        if !first.vol_short.is_finite()
            || !first.vol_long.is_finite()
            || !first.vol_blend.is_finite()
        {
            out.remove(0);
        } else {
            break;
        }
    }

    Ok(Json(out))
}
