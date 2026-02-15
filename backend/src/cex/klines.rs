use ::clickhouse::Client;
use anyhow::Result;
use futures::{StreamExt, TryStreamExt, stream::iter};
use std::env;
use std::time::Duration;
use tracing::{info, warn};

use crate::cex::common::CexMarketType;
use crate::db::{clickhouse, insert::insert_cex_klines};
use crate::exchanges::binance::api::{
    client::BinanceClient, endpoints::MarketType as BinanceMarketType,
};
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
    let exchange = exchange_name.to_ascii_lowercase();
    let max_retries = http_retry_attempts();

    loop {
        let (raw_bytes, delay_ms) = match exchange.as_str() {
            "binance" => {
                let mut attempt = 0usize;
                let bytes = loop {
                    let resp = BinanceClient::new()
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
                        .await;
                    match resp {
                        Ok(b) => break b,
                        Err(e) if is_rate_limited(&e) && attempt < max_retries => {
                            attempt += 1;
                            let backoff_ms = retry_backoff_ms(attempt);
                            warn!(
                                "binance klines rate-limited symbol={} interval={} attempt={}/{} backoff_ms={}",
                                api_symbol, interval, attempt, max_retries, backoff_ms
                            );
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        }
                        Err(e) => return Err(e.into()),
                    }
                };
                (bytes, 100u64)
            }
            "bybit" => {
                let mut attempt = 0usize;
                let bytes = loop {
                    let resp = BybitClient::new()
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
                        .await;
                    match resp {
                        Ok(b) => break b,
                        Err(e) if is_rate_limited(&e) && attempt < max_retries => {
                            attempt += 1;
                            let backoff_ms = retry_backoff_ms(attempt);
                            warn!(
                                "bybit klines rate-limited symbol={} interval={} attempt={}/{} backoff_ms={}",
                                api_symbol, interval, attempt, max_retries, backoff_ms
                            );
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                        }
                        Err(e) => return Err(e.into()),
                    }
                };
                (bytes, 150u64)
            }
            _ => anyhow::bail!("Kline sync not supported for CEX '{}'", exchange_name),
        };

        let klines_batch = match exchange.as_str() {
            "binance" => parse_binance_klines(&raw_bytes, market_symbol)?,
            "bybit" => parse_bybit_klines(&raw_bytes, market_symbol)?,
            _ => unreachable!(),
        };

        if klines_batch.is_empty() {
            break;
        }

        current_start =
            (klines_batch.last().unwrap().open_time.timestamp_millis() as u64).saturating_add(1);
        all_klines.extend(klines_batch);

        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    Ok(all_klines)
}

pub async fn sync_klines_with_spec(
    client: &Client,
    exchange_name: &str,
    market_type: CexMarketType,
    interval: &str,
    time_spec: TimeSpec,
) -> Result<()> {
    let exchange_name = exchange_name.trim().to_ascii_lowercase();
    info!(
        "Starting CEX kline sync for {} ({:?}) interval {}",
        exchange_name, market_type, interval
    );

    if interval != "1d" && interval != "1h" {
        anyhow::bail!("Unsupported kline interval: {}", interval);
    }

    let max_concurrency = env::var("CEX_SYNC_CONC_MARKETS")
        .or_else(|_| env::var("SYNC_CONC_MARKETS"))
        .unwrap_or_else(|_| "4".to_string())
        .parse::<usize>()
        .unwrap_or(4)
        .clamp(1, 8);

    let exchange_id = clickhouse::cex_exchange_id(&exchange_name);
    let markets =
        clickhouse::list_active_cex_markets(client, exchange_id, market_type.as_str()).await?;

    let per_market_batches = iter(markets)
        .map(|(market_id, market_symbol)| {
            let client = client.clone();
            let exchange_name = exchange_name.to_string();
            let interval = interval.to_string();
            let time_spec = time_spec.clone();

            async move {
                let last_ts_ms =
                    clickhouse::latest_kline_ts_ms(&client, market_id, &interval).await?;
                let (start_ms, end_ms) = time_spec.resolve(last_ts_ms);

                if start_ms >= end_ms {
                    return Ok::<_, anyhow::Error>(Vec::new());
                }

                let klines = fetch_klines_for_market(
                    &exchange_name,
                    &market_symbol,
                    &market_symbol,
                    market_type,
                    &interval,
                    start_ms,
                    end_ms,
                )
                .await?;

                Ok(klines
                    .into_iter()
                    .map(|k| (market_id, k))
                    .collect::<Vec<_>>())
            }
        })
        .buffer_unordered(max_concurrency)
        .try_collect::<Vec<Vec<(i32, NormalizedKline)>>>()
        .await?;

    let all_klines: Vec<(i32, NormalizedKline)> =
        per_market_batches.into_iter().flatten().collect();

    if !all_klines.is_empty() {
        let count = all_klines.len();
        insert_cex_klines(client, all_klines, interval).await?;
        info!("Inserted {} kline records for {}", count, exchange_name);
    } else {
        info!("No new kline records for {}", exchange_name);
    }

    Ok(())
}

fn is_rate_limited(e: &reqwest::Error) -> bool {
    e.status().map(|s| s.as_u16() == 429).unwrap_or(false)
}

fn retry_backoff_ms(attempt: usize) -> u64 {
    let base = 500u64;
    let max_backoff = 10_000u64;
    (base.saturating_mul(1u64 << attempt.min(6) as u32)).min(max_backoff)
}

fn http_retry_attempts() -> usize {
    env::var("SYNC_HTTP_RETRIES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(6)
}
