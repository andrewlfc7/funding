use anyhow::{anyhow, Result};
use futures::{stream::iter, StreamExt, TryStreamExt};
use sqlx::PgPool;
use std::time::Duration;
use tracing::info;
use std::env;

use crate::exchanges::shared::time::TimeSpec;
use crate::cex::common::CexMarketType;
use crate::db::insert::insert_cex_trades;
use crate::exchanges::shared::types::NormalizedTrade;

use crate::exchanges::binance::api::{
    client::BinanceClient,
    endpoints::MarketType as BinanceMarketType,
};
use crate::exchanges::bybit::api::{
    client::BybitClient,
    endpoints::Category as BybitCategory,
};
use crate::exchanges::binance::handler::handler::parse_binance_trades;
use crate::exchanges::bybit::handler::handler::parse_bybit_trades;

/// Fetch trades for a single market by **market_symbol** (e.g., "BTCUSDT").
/// We use `market_symbol` both for the exchange API call and for tagging in parsing.
async fn fetch_trades_for_market(
    exchange_name: &str,
    market_symbol: &str,
    market_type: CexMarketType,
    start_ms: u64,
    end_ms: u64,
) -> Result<Vec<NormalizedTrade>> {
    let mut all_trades = Vec::new();
    let mut current_start = start_ms;
    let api_symbol = market_symbol; // IMPORTANT: use market_symbol directly

    loop {
        let (raw_bytes, delay_ms) = match exchange_name {
            "binance" => {
                let mt = if market_type == CexMarketType::Perps {
                    BinanceMarketType::UsdFutures
                } else {
                    BinanceMarketType::Spot
                };
                (
                    BinanceClient::new()
                        .get_historical_trades(
                            mt,
                            api_symbol,
                            Some(current_start),
                            Some(end_ms),
                            Some(1000),
                        )
                        .await?,
                    100u64,
                )
            }
            "bybit" => {
                let cat = if market_type == CexMarketType::Perps {
                    BybitCategory::Linear
                } else {
                    BybitCategory::Spot
                };
                (
                    BybitClient::new()
                        .get_historical_trades(
                            cat,
                            api_symbol,
                            Some(current_start),
                            Some(end_ms),
                            Some(1000),
                        )
                        .await?,
                    150u64,
                )
            }
            _ => {
                return Err(anyhow!(
                    "Trade sync not supported for CEX '{}'",
                    exchange_name
                ))
            }
        };

        // Parse into your normalized trade type, tagged with market_symbol.
        let trades_batch = match exchange_name {
            "binance" => parse_binance_trades(&raw_bytes, market_symbol)?,
            "bybit" => parse_bybit_trades(&raw_bytes, market_symbol)?,
            _ => unreachable!(),
        };

        if trades_batch.is_empty() {
            break;
        }

        // Advance the window to just after the last trade time we received.
        // (Assumes NormalizedTrade.trade_time is a chrono/time type.)
        current_start = trades_batch
            .last()
            .unwrap()
            .trade_time
            .timestamp_millis() as u64
            + 1;

        all_trades.extend(trades_batch);

        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }

    Ok(all_trades)
}

/// Sync trades for all markets of the given exchange + market_type within the given time_spec.
pub async fn sync_trades_with_spec(
    pool: &PgPool,
    exchange_name: &str,
    market_type: CexMarketType,
    time_spec: TimeSpec,
) -> Result<()> {
    info!(
        "Starting CEX trade sync for {} ({:?})",
        exchange_name, market_type
    );

    // Resolve the exchange id.
    let exch = sqlx::query!(
        "SELECT id FROM cex_exchanges WHERE name = $1",
        exchange_name
    )
    .fetch_one(pool)
    .await?;

    // Load ONLY id + market_symbol (never the base) so we can't accidentally pass "BTC".
    let markets = sqlx::query!(
        r#"
        SELECT id, market_symbol
        FROM cex_markets
        WHERE exchange_id = $1
          AND market_type = $2
          AND is_active = TRUE
        ORDER BY market_symbol
        "#,
        exch.id,
        market_type.as_str()
    )
    .fetch_all(pool)
    .await?;

    let max_concurrency = env::var("SYNC_CONC_MARKETS")
        .unwrap_or_else(|_| "4".to_string())
        .parse::<usize>()
        .unwrap_or(4);

    // Fetch per-market concurrently, bounded by max_concurrency.
    let per_market_batches = iter(markets)
        .map(|m| {
            let pool = pool.clone();
            let exchange_name = exchange_name.to_string();
            let time_spec = time_spec.clone();

            async move {

                let last_ts_ms_opt = sqlx::query_scalar!(
                    "SELECT MAX(trade_time) FROM trades WHERE market_id = $1",
                    m.id
                )
                .fetch_one(&pool)
                .await?
                .map(|dt: time::OffsetDateTime| dt.unix_timestamp() * 1000); // i64

                let (start_ms, end_ms) = time_spec.resolve(last_ts_ms_opt);




                if start_ms >= end_ms {
                    // Nothing to pull for this market.
                    return Ok::<_, anyhow::Error>(Vec::new());
                }

                // CRITICAL: pass `m.market_symbol` for both API and labeling.
                let trades = fetch_trades_for_market(
                    &exchange_name,
                    &m.market_symbol,
                    market_type,
                    start_ms,
                    end_ms,
                )
                .await?;

                Ok(trades
                    .into_iter()
                    .map(|t| (m.id, t))
                    .collect::<Vec<(i32, NormalizedTrade)>>())
            }
        })
        .buffer_unordered(max_concurrency)
        .try_collect::<Vec<Vec<(i32, NormalizedTrade)>>>()
        .await?;

    let all_trades: Vec<(i32, NormalizedTrade)> =
        per_market_batches.into_iter().flatten().collect();

    if !all_trades.is_empty() {
        let count = all_trades.len();
        insert_cex_trades(pool, all_trades).await?;
        info!("Inserted {} trade records for {}", count, exchange_name);
    } else {
        info!("No new trade records for {}", exchange_name);
    }

    Ok(())
}
