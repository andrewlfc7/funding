use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{PgPool, Row};
use time::{Duration, OffsetDateTime};

// submodules
pub mod zscore_overview;
pub mod volatility_analysis;
pub mod cross_asset_matrix;
pub mod leaders_laggards;
pub mod inter_asset_zscore;
pub mod vol_liquidity;
pub mod microstructure_flow;


// -------- Timeframe + period parsing --------
#[derive(Clone, Copy, Debug)]
pub enum Tf { H1, H4, D1 }
impl Tf {
    pub fn from_str(s: &str) -> Option<Self> {
        match s { "1h" => Some(Tf::H1), "4h" => Some(Tf::H4), "1d" => Some(Tf::D1), _ => None }
    }
    pub fn period_secs(&self) -> i64 { match self { Tf::H1 => 3600, Tf::H4 => 4*3600, Tf::D1 => 24*3600 } }
    pub fn steps_per_day(&self) -> usize { match self { Tf::H1 => 24, Tf::H4 => 6, Tf::D1 => 1 } }
}
pub fn parse_period_days(s: &str) -> i64 {
    match s { "7d" => 7, "30d" => 30, "90d" => 90, "120d" => 120, _ => 30 }
}

// -------- Query helper: "A,B" or ?coins=A&coins=B --------
pub fn de_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where D: Deserializer<'de> {
    use serde::de::{Error, SeqAccess, Visitor};
    use std::fmt;
    struct StrOrVec;
    impl<'de> Visitor<'de> for StrOrVec {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result { f.write_str("string or sequence of strings") }
        fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new(); while let Some(x) = seq.next_element::<String>()? { out.push(x); } Ok(out)
        }
    }
    deserializer.deserialize_any(StrOrVec)
}

// -------- DB-aware market + OHLCV (matches your schema) --------
async fn exchange_id_by_name(pool: &PgPool, name: &str) -> anyhow::Result<i32> {
    let r = sqlx::query("SELECT id FROM cex_exchanges WHERE lower(name) = lower($1)")
        .bind(name)
        .fetch_one(pool)
        .await?;
    Ok(r.get::<i32, _>("id"))
}

/// Pick one market (exchange, market_type, symbol==BASE) that HAS hourly rows in the window.
/// Prefer quote_asset in ('USDT','USD','USDC').
async fn resolve_market_id_with_data(
    pool: &PgPool,
    exchange_name: &str,
    base_symbol: &str,
    market_type: &str,              // "spot" | "perps"
    since_unix: i64,
) -> anyhow::Result<i32> {
    let ex_id = exchange_id_by_name(pool, exchange_name).await?;
    let base_up = base_symbol.to_uppercase();

    if let Some(r) = sqlx::query(
        r#"
        SELECT m.id
        FROM cex_markets m
        WHERE m.exchange_id = $1
          AND m.market_type = $2
          AND m.symbol = $3
          AND m.is_active = TRUE
          AND m.quote_asset IN ('USDT','USD','USDC')
          AND EXISTS (
              SELECT 1 FROM klines_hourly k
              WHERE k.market_id = m.id AND k.time >= to_timestamp($4)
          )
        ORDER BY
          CASE m.quote_asset WHEN 'USDT' THEN 0 WHEN 'USD' THEN 1 WHEN 'USDC' THEN 2 ELSE 3 END
        LIMIT 1
        "#
    )
    .bind(ex_id).bind(market_type).bind(&base_up).bind(since_unix)
    .fetch_optional(pool).await? {
        return Ok(r.get::<i32, _>("id"));
    }

    let r = sqlx::query(
        r#"
        SELECT m.id
        FROM cex_markets m
        WHERE m.exchange_id = $1
          AND m.market_type = $2
          AND m.symbol = $3
          AND m.is_active = TRUE
          AND EXISTS (
              SELECT 1 FROM klines_hourly k
              WHERE k.market_id = m.id AND k.time >= to_timestamp($4)
          )
        ORDER BY m.quote_asset ASC, m.id ASC
        LIMIT 1
        "#
    )
    .bind(ex_id).bind(market_type).bind(&base_up).bind(since_unix)
    .fetch_one(pool).await?;

    Ok(r.get::<i32, _>("id"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ohlcv {
    pub ts: i64,   // epoch seconds (UTC bucket start)
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
}

async fn fetch_hourly_by_market(pool: &PgPool, market_id: i32, since_unix: i64) -> anyhow::Result<Vec<Ohlcv>> {
    let rows = sqlx::query(
        r#"
        SELECT
          EXTRACT(EPOCH FROM time)::bigint AS ts,
          close::double precision  AS close,
          high::double precision   AS high,
          low::double precision    AS low,
          volume::double precision AS volume
        FROM klines_hourly
        WHERE market_id = $1 AND time >= to_timestamp($2)
        ORDER BY time ASC
        "#
    )
    .bind(market_id).bind(since_unix)
    .fetch_all(pool).await?;

    Ok(rows.into_iter().map(|r| Ohlcv {
        ts: r.get("ts"),
        close: r.get("close"),
        high: r.get("high"),
        low: r.get("low"),
        volume: r.get("volume"),
    }).collect())
}

fn floor_to(ts: i64, period: i64) -> i64 { ts - (ts % period) }

/// Public so endpoints can reuse for multi-coin.
pub fn resample_from_hourly(hourly: &[Ohlcv], period_secs: i64) -> Vec<Ohlcv> {
    if hourly.is_empty() || period_secs <= 3600 { return hourly.to_vec(); }
    let mut out = Vec::new();
    let mut bucket = floor_to(hourly[0].ts, period_secs);
    let mut hi = f64::NEG_INFINITY;
    let mut lo = f64::INFINITY;
    let mut vol = 0.0;
    let mut cls = hourly[0].close;

    for bar in hourly {
        let b = floor_to(bar.ts, period_secs);
        if b != bucket {
            out.push(Ohlcv { ts: bucket, close: cls, high: hi, low: lo, volume: vol });
            bucket = b; hi = f64::NEG_INFINITY; lo = f64::INFINITY; vol = 0.0;
        }
        if bar.high > hi { hi = bar.high; }
        if bar.low  < lo { lo  = bar.low; }
        vol += bar.volume;
        cls = bar.close;
    }
    out.push(Ohlcv { ts: bucket, close: cls, high: hi, low: lo, volume: vol });
    out
}

pub async fn get_ohlcv_resampled(
    pool: &PgPool,
    exchange: &str,
    base_symbol: &str,
    market_type: &str, // "spot" | "perps"
    tf: Tf,
    days: i64,
) -> anyhow::Result<Vec<Ohlcv>> {
    let since_unix = (OffsetDateTime::now_utc() - Duration::days(days)).unix_timestamp();
    let market_id = resolve_market_id_with_data(pool, exchange, base_symbol, market_type, since_unix).await?;
    let hourly = fetch_hourly_by_market(pool, market_id, since_unix).await?;
    Ok(resample_from_hourly(&hourly, tf.period_secs()))
}

// -------- math (EWMA; basic returns/vol/z) --------
pub fn pct_returns(xs: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(xs.len()); if xs.is_empty() { return out; }
    out.push(0.0);
    for w in xs.windows(2) { let (p0,p1)=(w[0],w[1]); out.push(if p0!=0.0 {(p1/p0)-1.0} else {0.0}); }
    out
}
pub fn log_returns(xs: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(xs.len()); if xs.is_empty() { return out; }
    out.push(0.0);
    for w in xs.windows(2) { let (p0,p1)=(w[0],w[1]); out.push(if p0>0.0 && p1>0.0 {(p1/p0).ln()} else {0.0}); }
    out
}
pub fn ewma_alpha(values: &[f64], alpha: f64) -> Vec<f64> {
    assert!((0.0..=1.0).contains(&alpha) && alpha>0.0);
    if values.is_empty() { return vec![]; }
    let mut out = Vec::with_capacity(values.len());
    let mut s = values[0]; out.push(s);
    for &x in &values[1..] { s = alpha * x + (1.0 - alpha) * s; out.push(s); }
    out
}
pub fn ewma_span(values: &[f64], span: usize) -> Vec<f64> {
    let span = span.max(1); let alpha = 2.0 / (span as f64 + 1.0); ewma_alpha(values, alpha)
}
pub fn rolling_mean_std(xs: &[f64], win: usize) -> (Vec<f64>, Vec<f64>) {
    let n = xs.len(); let mut m = vec![f64::NAN; n]; let mut s = vec![f64::NAN; n];
    if win == 0 || n == 0 { return (m, s); }
    for i in 0..n {
        if i + 1 >= win {
            let sl = &xs[i + 1 - win..=i];
            let mu = sl.iter().sum::<f64>() / win as f64;
            let var = sl.iter().map(|v| (v - mu)*(v - mu)).sum::<f64>() / (win as f64).max(1.0);
            m[i] = mu; s[i] = var.sqrt();
        }
    }
    (m, s)
}
pub fn zscore_series(xs: &[f64], win: usize) -> Vec<f64> {
    let (m, s) = rolling_mean_std(xs, win);
    xs.iter().enumerate().map(|(i, &v)| {
        let sd = s[i];
        if sd.is_finite() && sd > 0.0 && m[i].is_finite() { (v - m[i]) / sd } else { f64::NAN }
    }).collect()
}
pub fn histogram_counts(xs: &[f64], bucket_centers: &[f64]) -> Vec<usize> {
    if bucket_centers.is_empty() { return vec![]; }
    let step = if bucket_centers.len()>1 { bucket_centers[1] - bucket_centers[0] } else { 1.0 };
    let half = step/2.0; let mut c = vec![0usize; bucket_centers.len()];
    for &x in xs {
        for (i,&b) in bucket_centers.iter().enumerate() {
            if x >= b - half && x < b + half { c[i]+=1; break; }
        }
    }
    c
}
pub fn lag(xs: &[f64], k: usize) -> Vec<f64> {
    if xs.is_empty() { return vec![]; }
    let mut out = vec![f64::NAN; xs.len()]; for i in k..xs.len() { out[i] = xs[i-k]; } out
}

// =======================
// Universe selection & multi-fetch
// =======================
use std::collections::HashMap;

/// Return (symbol, market_id, sym_usd_volume) for the top N symbols by USD notional
/// over the last `days` for (exchange, market_type). Picks the *best* market per symbol
/// (highest usd_volume; tie-breaker prefers USDT>USD>USDC).
pub async fn top_markets_by_usd_volume_live(
    pool: &PgPool,
    exchange_name: &str,
    market_type: &str, // "spot" | "perps"
    days: i32,         // 7 | 30 | 90 | 120 ...
    top_n: i64,        // 30 | 50
) -> anyhow::Result<Vec<(String, i32, f64)>> {
    let rows = sqlx::query(
        r#"
WITH base AS (
  SELECT e.id AS exchange_id,
         m.id AS market_id,
         m.symbol,
         m.quote_asset,
         SUM(k.close::double precision * k.volume::double precision) AS usd_volume
  FROM cex_exchanges e
  JOIN cex_markets   m ON m.exchange_id = e.id
  JOIN klines_hourly k ON k.market_id   = m.id
  WHERE lower(e.name) = lower($1)
    AND m.market_type = $2
    AND m.is_active = TRUE
    AND m.quote_asset IN ('USD','USDT','USDC')
    AND k.time >= now() - make_interval(days => $3::int)
  GROUP BY e.id, m.id, m.symbol, m.quote_asset
),
sym_sum AS (
  SELECT symbol, SUM(usd_volume) AS sym_usd_volume
  FROM base
  GROUP BY symbol
),
best_market AS (
  SELECT b.symbol, b.market_id, b.quote_asset, b.usd_volume,
         ROW_NUMBER() OVER (
           PARTITION BY b.symbol
           ORDER BY b.usd_volume DESC,
                    CASE b.quote_asset WHEN 'USDT' THEN 0 WHEN 'USD' THEN 1 WHEN 'USDC' THEN 2 ELSE 3 END
         ) AS rnk
  FROM base b
),
top_syms AS (
  SELECT s.symbol, s.sym_usd_volume
  FROM sym_sum s
  ORDER BY s.sym_usd_volume DESC
  LIMIT $4
)
SELECT t.symbol, bm.market_id, t.sym_usd_volume
FROM top_syms t
JOIN best_market bm ON bm.symbol = t.symbol AND bm.rnk = 1
ORDER BY t.sym_usd_volume DESC
        "#
    )
    .bind(exchange_name)
    .bind(market_type)
    .bind(days)
    .bind(top_n)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let symbol: String = r.get("symbol");
            let market_id: i32 = r.get("market_id");
            let sym_usd_volume: f64 = r.get::<f64, _>("sym_usd_volume");
            (symbol, market_id, sym_usd_volume)
        })
        .collect())
}

/// Fetch hourly OHLCV for multiple market_ids in one query.
pub async fn fetch_multi_hourly_ohlcv(
    pool: &PgPool,
    market_ids: &[i32],
    since_unix: i64,
) -> anyhow::Result<HashMap<i32, Vec<Ohlcv>>> {
    if market_ids.is_empty() { return Ok(HashMap::new()); }
    let rows = sqlx::query(
        r#"
        SELECT
          market_id,
          EXTRACT(EPOCH FROM time)::bigint AS ts,
          close::double precision  AS close,
          high::double precision   AS high,
          low::double precision    AS low,
          volume::double precision AS volume
        FROM klines_hourly
        WHERE market_id = ANY($1) AND time >= to_timestamp($2)
        ORDER BY market_id ASC, time ASC
        "#
    )
    .bind(market_ids)
    .bind(since_unix)
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<i32, Vec<Ohlcv>> = HashMap::new();
    for r in rows {
        let mid: i32 = r.get("market_id");
        map.entry(mid).or_default().push(Ohlcv{
            ts: r.get("ts"),
            close: r.get("close"),
            high: r.get("high"),
            low: r.get("low"),
            volume: r.get("volume"),
        });
    }
    Ok(map)
}



