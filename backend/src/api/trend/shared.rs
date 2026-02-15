use anyhow::Result;
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct DailyBar {
    pub ts_ms: i64, // epoch ms at midnight UTC
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
}

// one preferred market per base symbol (highest USD notional; tie USDT>USD>USDC)
pub async fn fetch_universe_daily(
    pool: &PgPool,
    exchange: &str,
    market_type: &str,
) -> Result<BTreeMap<String, Vec<DailyBar>>> {
    let rows = sqlx::query(
        r#"
WITH base AS (
  SELECT m.id AS market_id,
         m.symbol,
         m.quote_asset,
         SUM(k.close::double precision * k.volume::double precision) AS usd
  FROM cex_exchanges e
  JOIN cex_markets   m ON m.exchange_id = e.id
  JOIN klines_daily  k ON k.market_id   = m.id
  WHERE lower(e.name) = lower($1)
    AND m.market_type = $2
    AND m.is_active   = TRUE
    AND k.close  IS NOT NULL
    AND k.volume IS NOT NULL
  GROUP BY 1,2,3
),
best AS (
  SELECT market_id
  FROM (
    SELECT market_id, symbol, quote_asset, usd,
           ROW_NUMBER() OVER (
             PARTITION BY symbol
             ORDER BY usd DESC,
                      CASE quote_asset
                        WHEN 'USDT' THEN 0
                        WHEN 'USD'  THEN 1
                        WHEN 'USDC' THEN 2
                        ELSE 3
                      END
           ) rn
    FROM base
  ) s WHERE rn = 1
),
k_raw AS (
  SELECT m.symbol AS base_symbol,
         k.date::date AS d,
         k.close::float8  AS close,
         k.high::float8   AS high,
         k.low::float8    AS low,
         k.volume::float8 AS volume
  FROM klines_daily k
  JOIN best b        ON b.market_id = k.market_id
  JOIN cex_markets m ON m.id        = k.market_id
  WHERE k.close  IS NOT NULL
    AND k.volume IS NOT NULL
)
SELECT
  base_symbol,
  (EXTRACT(EPOCH FROM d) * 1000)::bigint AS ts_ms,
  close, high, low, volume
FROM k_raw
ORDER BY d ASC, base_symbol ASC
        "#,
    )
    .bind(exchange)
    .bind(market_type)
    .fetch_all(pool)
    .await?;

    let mut by_sym: BTreeMap<String, Vec<DailyBar>> = BTreeMap::new();
    for r in rows {
        let sym: String = r.get("base_symbol");
        by_sym.entry(sym).or_default().push(DailyBar {
            ts_ms: r.get::<i64, _>("ts_ms"),
            close: r.get::<f64, _>("close"),
            high: r.get::<f64, _>("high"),
            low: r.get::<f64, _>("low"),
            volume: r.get::<f64, _>("volume"),
        });
    }
    Ok(by_sym)
}

pub async fn fetch_single_daily(
    pool: &PgPool,
    exchange: &str,
    market_type: &str,
    base: &str,
) -> Result<Vec<DailyBar>> {
    let rows = sqlx::query(
        r#"
WITH preferred AS (
  SELECT m.id AS market_id
  FROM cex_exchanges e
  JOIN cex_markets   m ON m.exchange_id = e.id
  WHERE lower(e.name) = lower($1)
    AND m.market_type = $2
    AND m.is_active   = TRUE
    AND UPPER(m.market_symbol) IN (
      UPPER($3) || 'USDT', UPPER($3) || 'USD', UPPER($3) || 'USDC',
      UPPER($3) || 'FDUSD', UPPER($3) || 'BUSD', UPPER($3) || 'TUSD',
      UPPER($3) || 'DAI',   UPPER($3) || 'USDP'
    )
  ORDER BY CASE
     WHEN UPPER(m.market_symbol) = UPPER($3)||'USDT' THEN 1
     WHEN UPPER(m.market_symbol) = UPPER($3)||'USD'  THEN 2
     WHEN UPPER(m.market_symbol) = UPPER($3)||'USDC' THEN 3
     ELSE 99
  END
  LIMIT 1
)
SELECT
  (EXTRACT(EPOCH FROM k.date) * 1000)::bigint AS ts_ms,
  k.close::float8  AS close,
  k.high::float8   AS high,
  k.low::float8    AS low,
  k.volume::float8 AS volume
FROM klines_daily k
JOIN preferred p ON p.market_id = k.market_id
WHERE k.close  IS NOT NULL
  AND k.volume IS NOT NULL
ORDER BY k.date ASC
        "#,
    )
    .bind(exchange)
    .bind(market_type)
    .bind(base)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| DailyBar {
            ts_ms: r.get("ts_ms"),
            close: r.get("close"),
            high: r.get("high"),
            low: r.get("low"),
            volume: r.get("volume"),
        })
        .collect())
}
