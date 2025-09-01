// src/cex/klines.rs
use anyhow::{anyhow, Result};
use futures::{stream::iter, StreamExt, TryStreamExt};
use sqlx::PgPool;
use std::time::Duration;
use tracing::info;
use std::env;

use crate::cex::common::CexMarketType;
use crate::db::insert::insert_cex_klines;
use crate::exchanges::binance::api::{client::BinanceClient, endpoints::MarketType as BinanceMarketType};
use crate::exchanges::binance::handler::handler::parse_binance_klines;
use crate::exchanges::bybit::api::{client::BybitClient, endpoints::Category as BybitCategory};
use crate::exchanges::bybit::handler::handler::parse_bybit_klines;
use crate::exchanges::shared::time::TimeSpec;
use crate::exchanges::shared::types::NormalizedKline;

async fn fetch_klines_for_market(
    exchange_name: &str,
    market_symbol: &str,
    api_symbol: &str,
    market_type: CexMarketType,
    interval: &str,
    start_ms: u64,
    end_ms: u64,
) -> Result<Vec<NormalizedKline>> {
    let mut all_klines = Vec::new();
    let mut current_start = start_ms;

    loop {
        let (raw_bytes, delay_ms) = match exchange_name {
            "binance" => (
                BinanceClient::new()
                    .get_klines(
                        if market_type == CexMarketType::Perps {
                            BinanceMarketType::UsdFutures
                        } else {
                            BinanceMarketType::Spot
                        },
                        api_symbol,
                        interval,
                        Some(current_start),
                        Some(end_ms),
                        Some(1000),
                    )
                    .await?,
                100u64,
            ),
            "bybit" => (
                BybitClient::new()
                    .get_klines(
                        if market_type == CexMarketType::Perps {
                            BybitCategory::Linear
                        } else {
                            BybitCategory::Spot
                        },
                        api_symbol,
                        interval,
                        Some(current_start),
                        Some(end_ms),
                        Some(1000),
                    )
                    .await?,
                150u64,
            ),
            _ => anyhow::bail!("Kline sync not supported for CEX '{}'", exchange_name),
        };

        let klines_batch = match exchange_name {
            "binance" => parse_binance_klines(&raw_bytes, market_symbol)?,
            "bybit" => parse_bybit_klines(&raw_bytes, market_symbol)?,
            _ => unreachable!(),
        };

        if klines_batch.is_empty() {
            break;
        }

        // advance just past the last bar’s open_time to avoid duplicates
        current_start = (klines_batch.last().unwrap().open_time.timestamp_millis() as u64).saturating_add(1);
        all_klines.extend(klines_batch);

        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    Ok(all_klines)
}



pub async fn sync_klines_with_spec(
    pool: &PgPool,
    exchange_name: &str,
    market_type: CexMarketType,
    interval: &str,
    time_spec: TimeSpec,
) -> Result<()> {
    info!(
        "Starting CEX kline sync for {} ({:?}) interval {}",
        exchange_name, market_type, interval
    );

    // Table + primary time column for the interval
    let (table_name, time_col) = match interval {
        "1d" => ("klines_daily", "date"),   // DATE column
        "1h" => ("klines_hourly", "time"),  // TIMESTAMPTZ column
        _ => anyhow::bail!("Unsupported kline interval: {}", interval),
    };

    let max_concurrency = env::var("SYNC_CONC_MARKETS")
        .unwrap_or_else(|_| "4".to_string())
        .parse::<usize>()
        .unwrap_or(4);

    // Resolve exchange id
    let exch = sqlx::query!(
        "SELECT id FROM cex_exchanges WHERE name = $1",
        exchange_name
    )
    .fetch_one(pool)
    .await?;

    // Active markets for that exchange + market type
    let markets = sqlx::query!(
        r#"
        SELECT id, market_symbol
        FROM cex_markets
        WHERE exchange_id = $1
          AND market_type  = $2
          AND is_active    = TRUE
        "#,
        exch.id,
        market_type.as_str()
    )
    .fetch_all(pool)
    .await?;

    // For each market: read last time (nullable) -> resolve range -> fetch -> stage rows
    let per_market_batches = iter(markets)
        .map(|m| {
            let pool = pool.clone();
            let exchange_name = exchange_name.to_string();
            let interval = interval.to_string();
            let time_spec = time_spec.clone();
            let table_name = table_name.to_string();
            let time_col = time_col.to_string();

            async move {
                let query_string = format!(
                    r#"SELECT MAX({col}) FROM {table} WHERE market_id = $1"#,
                    col = time_col,
                    table = table_name
                );

                // Compute last_ts_ms based on the interval's column type
                let last_ts_ms: Option<i64> = match interval.as_str() {
                    "1d" => {
                        // DATE -> start of day UTC
                        let last_date: Option<time::Date> = sqlx::query_scalar(&query_string)
                            .bind(m.id)
                            .fetch_one(&pool)
                            .await?;
                        last_date.map(|d| {
                            let dt = d.with_time(time::Time::MIDNIGHT).assume_utc();
                            dt.unix_timestamp() * 1000
                        })
                    }
                    "1h" => {
                        // TIMESTAMPTZ
                        let last_dt: Option<time::OffsetDateTime> = sqlx::query_scalar(&query_string)
                            .bind(m.id)
                            .fetch_one(&pool)
                            .await?;
                        last_dt.map(|dt| dt.unix_timestamp() * 1000)
                    }
                    _ => unreachable!(),
                };

                let (start_ms, end_ms) = time_spec.resolve(last_ts_ms);
                if start_ms >= end_ms {
                    return Ok::<_, anyhow::Error>(Vec::new());
                }

                let klines = fetch_klines_for_market(
                        &exchange_name,
                        &m.market_symbol,
                        &m.market_symbol,
                        market_type,
                        &interval,
                        start_ms,
                        end_ms
                    ).await?;

                Ok(klines.into_iter().map(|k| (m.id, k)).collect::<Vec<_>>())
            }
        })
        .buffer_unordered(max_concurrency)
        .try_collect::<Vec<Vec<(i32, NormalizedKline)>>>()
        .await?;

    let all_klines: Vec<(i32, NormalizedKline)> = per_market_batches.into_iter().flatten().collect();

    if !all_klines.is_empty() {
        let count = all_klines.len();
        insert_cex_klines(pool, all_klines, interval).await?;
        info!("Inserted {} kline records for {}", count, exchange_name);
    } else {
        info!("No new kline records for {}", exchange_name);
    }

    Ok(())
}
