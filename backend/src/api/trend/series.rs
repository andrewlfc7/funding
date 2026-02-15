use axum::{
    Json,
    extract::{Query, State},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/* ---------- query params ---------- */

#[derive(Debug, Deserialize)]
pub struct SeriesParams {
    pub exchange: String,
    pub market_type: String, // "spot" | "perps"
    #[serde(alias = "base", alias = "ticker")]
    pub symbol: String, // e.g. "BTC"
    pub days: Option<i64>,   // default 120
    pub span: Option<usize>, // for EWMA endpoints (default 30)
    pub annualize: Option<bool>, // for realized vol (default false)
}

/* ---------- DTOs ---------- */

#[derive(Debug, Serialize)]
pub struct Point {
    pub date: String,
    pub value: f64,
}

#[derive(Debug, Serialize)]
pub struct EwmaVolumePoint {
    pub date: String,
    pub volume_ewma: f64,
    pub dollar_volume_ewma: f64,
}

/* ---------- helpers ---------- */

fn parse_market_type(s: &str) -> Result<&'static str, String> {
    match s {
        "spot" => Ok("spot"),
        "perps" => Ok("perps"),
        other => Err(format!(
            "invalid market_type '{}'; use 'spot' or 'perps'",
            other
        )),
    }
}

fn ewma_span(values: &[f64], span: usize) -> Vec<f64> {
    let span = span.max(1);
    let alpha = 2.0 / (span as f64 + 1.0);
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

fn stddev(xs: &[f64]) -> f64 {
    let n = xs.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let mean = xs.iter().sum::<f64>() / n;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    var.sqrt()
}

/* ---------- common fetch (daily OHLCV) ---------- */

#[derive(sqlx::FromRow)]
struct KRow {
    ts: i64, // epoch ms
    close: f64,
    volume: f64,
}

async fn fetch_daily_rows(
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
          k.close::float8  AS "close!",
          k.volume::float8 AS "volume!"
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

pub async fn get_volume_ewma30(
    State(pool): State<PgPool>,
    Query(q): Query<SeriesParams>,
) -> Result<Json<Vec<EwmaVolumePoint>>, (axum::http::StatusCode, String)> {
    let days = q.days.unwrap_or(180).max(1);
    let span = q.span.unwrap_or(30).max(1);
    let rows = fetch_daily_rows(&pool, &q.exchange, &q.market_type, &q.symbol, days).await?;

    let ts: Vec<i64> = rows.iter().map(|r| r.ts).collect();
    let vols: Vec<f64> = rows.iter().map(|r| r.volume).collect();
    let cls: Vec<f64> = rows.iter().map(|r| r.close).collect();
    let dv: Vec<f64> = vols.iter().zip(cls.iter()).map(|(v, c)| v * c).collect();

    let volume_ewma = ewma_span(&vols, span);
    let dollar_volume_ewma = ewma_span(&dv, span);

    let out: Vec<EwmaVolumePoint> = (0..ts.len())
        .map(|i| {
            let dt = NaiveDateTime::from_timestamp_millis(ts[i]).unwrap_or_default();
            EwmaVolumePoint {
                date: dt.format("%Y-%m-%d").to_string(),
                volume_ewma: volume_ewma[i],
                dollar_volume_ewma: dollar_volume_ewma[i],
            }
        })
        .collect();

    Ok(Json(out))
}

pub async fn get_vol_daily_realized(
    State(pool): State<PgPool>,
    Query(q): Query<SeriesParams>,
) -> Result<Json<Vec<Point>>, (axum::http::StatusCode, String)> {
    let days = q.days.unwrap_or(180).max(2);
    let ann = q.annualize.unwrap_or(false);
    let rows = fetch_daily_rows(&pool, &q.exchange, &q.market_type, &q.symbol, days).await?;

    // Calculate daily returns
    let mut rets: Vec<(i64, f64)> = Vec::with_capacity(rows.len().saturating_sub(1));
    for window in rows.windows(2) {
        let p0 = window[0].close;
        let p1 = window[1].close;
        let ts = window[1].ts;
        let ret = if p0 > 0.0 { (p1 / p0) - 1.0 } else { 0.0 };
        rets.push((ts, ret));
    }

    // Calculate daily realized volatility (absolute return)
    let out: Vec<Point> = rets
        .into_iter()
        .map(|(ts, ret)| {
            let mut daily_vol = ret.abs();
            if ann {
                daily_vol *= (365.0_f64).sqrt(); // Annualize if requested
            }
            let dt = NaiveDateTime::from_timestamp_millis(ts).unwrap_or_default();
            Point {
                date: dt.format("%Y-%m-%d").to_string(),
                value: daily_vol,
            }
        })
        .collect();

    Ok(Json(out))
}

pub async fn get_returns_daily(
    State(pool): State<PgPool>,
    Query(q): Query<SeriesParams>,
) -> Result<Json<Vec<Point>>, (axum::http::StatusCode, String)> {
    let days = q.days.unwrap_or(120).max(2);
    let rows = fetch_daily_rows(&pool, &q.exchange, &q.market_type, &q.symbol, days).await?;

    if rows.len() < 2 {
        return Ok(Json(vec![]));
    }

    let mut out = Vec::with_capacity(rows.len() - 1);
    let mut prev: Option<f64> = None;

    for r in rows {
        if let Some(pc) = prev {
            let ret = if pc > 0.0 { (r.close / pc) - 1.0 } else { 0.0 };
            let dt = NaiveDateTime::from_timestamp_millis(r.ts).unwrap_or_default();
            out.push(Point {
                date: dt.format("%Y-%m-%d").to_string(),
                value: ret,
            });
        }
        prev = Some(r.close);
    }
    Ok(Json(out))
}

pub async fn get_price_close(
    State(pool): State<PgPool>,
    Query(q): Query<SeriesParams>,
) -> Result<Json<Vec<Point>>, (axum::http::StatusCode, String)> {
    let days = q.days.unwrap_or(120).max(1);
    let rows = fetch_daily_rows(&pool, &q.exchange, &q.market_type, &q.symbol, days).await?;

    let out: Vec<Point> = rows
        .into_iter()
        .map(|r| {
            let dt = NaiveDateTime::from_timestamp_millis(r.ts).unwrap_or_default();
            Point {
                date: dt.format("%Y-%m-%d").to_string(),
                value: r.close,
            }
        })
        .collect();
    Ok(Json(out))
}
