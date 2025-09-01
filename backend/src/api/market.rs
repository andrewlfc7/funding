use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ---------- Params ----------

#[derive(Debug, Deserialize)]
pub struct RvParams {
    pub exchange: String,
    pub market_type: String,                 // "spot" | "perps"
    #[serde(alias = "market_symbol", alias = "base", alias = "ticker")]
    pub symbol: String,                      // e.g. "BTC"
    pub days: Option<i64>,
    pub vol_window: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct KlinesParams {
    pub exchange: String,
    pub market_type: String,                 // "spot" | "perps"
    #[serde(alias = "market_symbol", alias = "base", alias = "ticker")]
    pub symbol: String,                      // e.g. "BTC"
    pub days: Option<i64>,
}




// ---------- DTOs ----------

#[derive(Debug, Serialize)]
pub struct KlineDTO {
    pub ts: i64, // epoch millis
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Serialize)]
pub struct SimplePoint {
    pub ts: i64,    // epoch millis
    pub value: f64, // return OR volatility depending on endpoint
}

// ---------- Router ----------

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/api/klines/daily", get(get_daily_klines))
        .route("/api/signals/returns", get(get_returns))
        .route("/api/signals/volatility", get(get_volatility))
}

// ---------- Handlers ----------
async fn get_daily_klines(
    State(pool): State<PgPool>,
    Query(q): Query<KlinesParams>,
) -> Result<Json<Vec<KlineDTO>>, (axum::http::StatusCode, String)> {
    let days = q.days.unwrap_or(90).max(1);
    let mt = parse_market_type(&q.market_type)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

    let rows = sqlx::query!(
        r#"
        WITH preferred AS (
          SELECT m.id AS market_id
          FROM cex_markets m
          JOIN cex_exchanges e ON e.id = m.exchange_id
          WHERE e.name = $1
            AND m.market_type = $2
            AND UPPER(m.market_symbol) IN (
              UPPER($3) || 'USDT',
              UPPER($3) || 'USDC',
              UPPER($3) || 'FDUSD',
              UPPER($3) || 'BUSD',
              UPPER($3) || 'TUSD',
              UPPER($3) || 'USD',
              UPPER($3) || 'DAI',
              UPPER($3) || 'USDP'
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
            k.open::float8   AS "open!",
            k.high::float8   AS "high!",
            k.low::float8    AS "low!",
            k.close::float8  AS "close!",
            k.volume::float8 AS "volume!"
        FROM klines_daily k
        JOIN preferred p ON p.market_id = k.market_id
        WHERE k.date >= NOW() - ($4 * INTERVAL '1 day')
        ORDER BY k.date ASC
        "#,
        q.exchange, mt, q.symbol, days as f64
    )
    .fetch_all(&pool)
    .await
    .map_err(internal)?;

    let out = rows.into_iter().map(|r| KlineDTO {
        ts: r.ts, open: r.open, high: r.high, low: r.low, close: r.close, volume: r.volume
    }).collect();

    Ok(Json(out))
}



async fn get_returns(
    State(pool): State<PgPool>,
    Query(q): Query<RvParams>,
) -> Result<Json<Vec<SimplePoint>>, (axum::http::StatusCode, String)> {
    let days = q.days.unwrap_or(90).max(2);
    let mt = parse_market_type(&q.market_type)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

    let rows = sqlx::query!(
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
        q.exchange, mt, q.symbol, days as f64
    )
    .fetch_all(&pool)
    .await
    .map_err(internal)?;

    let mut out = Vec::<SimplePoint>::with_capacity(rows.len().saturating_sub(1));
    let mut prev: Option<f64> = None;

    for r in rows {
        let c = r.close;
        if let Some(pc) = prev {
            let ret = if pc > 0.0 { (c / pc) - 1.0 } else { 0.0 };
            out.push(SimplePoint { ts: r.ts, value: ret });
        }
        prev = Some(c);
    }

    Ok(Json(out))
}


async fn get_volatility(
    State(pool): State<PgPool>,
    Query(q): Query<RvParams>,
) -> Result<Json<Vec<SimplePoint>>, (axum::http::StatusCode, String)> {
    let days = q.days.unwrap_or(90).max(2);
    let win  = q.vol_window.unwrap_or(30).max(2);
    let mt = parse_market_type(&q.market_type)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))?;

    let rows = sqlx::query!(
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
        q.exchange, mt, q.symbol, days as f64
    )
    .fetch_all(&pool)
    .await
    .map_err(internal)?;

    // compute simple returns first
    let mut rets: Vec<(i64, f64)> = Vec::with_capacity(rows.len().saturating_sub(1));
    let mut prev: Option<f64> = None;

    for r in rows {
        if let Some(pc) = prev {
            let ret = if pc > 0.0 { (r.close / pc) - 1.0 } else { 0.0 };
            rets.push((r.ts, ret));
        }
        prev = Some(r.close);
    }

    // rolling stdev over `win`
    let mut out: Vec<SimplePoint> = Vec::new();
    for i in 0..rets.len() {
        if i + 1 >= win {
            let window = &rets[i + 1 - win..=i];
            let vals: Vec<f64> = window.iter().map(|(_, r)| *r).collect();
            out.push(SimplePoint { ts: rets[i].0, value: stddev(&vals) });
        }
    }

    Ok(Json(out))
}


// ---------- helpers ----------

fn parse_market_type(s: &str) -> Result<&'static str, String> {
    match s {
        "spot" => Ok("spot"),
        "perps" => Ok("perps"),
        other => Err(format!("invalid market_type '{}'; use 'spot' or 'perps'", other)),
    }
}

fn stddev(xs: &[f64]) -> f64 {
    let n = xs.len() as f64;
    if n < 2.0 { return 0.0; }
    let mean = xs.iter().copied().sum::<f64>() / n;
    let var = xs.iter().map(|x| {
        let d = x - mean;
        d * d
    }).sum::<f64>() / (n - 1.0);
    var.sqrt()
}

fn internal<E: std::fmt::Display>(e: E) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
