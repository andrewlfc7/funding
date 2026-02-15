use ::clickhouse::Client;
use anyhow::{Context, Result, anyhow};
use tracing::info;

use crate::cex::common::CexMarketType;
use crate::db::insert::{upsert_cex_exchange, upsert_cex_markets};
use crate::exchanges::shared::types::CexMarket;

use crate::exchanges::binance::api::{
    client::BinanceClient, endpoints::MarketType as BinanceMarketType,
};
use crate::exchanges::binance::handler::handler::parse_binance_markets;
use crate::exchanges::bybit::api::{client::BybitClient, endpoints::Category as BybitCategory};
use crate::exchanges::bybit::handler::handler::parse_bybit_markets;

fn keep_selected_quote(m: &CexMarket, selected: &str) -> bool {
    let sel = selected.to_ascii_uppercase();
    let by_suffix = m.market_symbol.to_ascii_uppercase().ends_with(&sel);
    let by_quote = m.quote_currency.to_ascii_uppercase() == sel;
    by_suffix || by_quote
}

fn is_stablecoin_symbol(symbol: &str) -> bool {
    let stablecoins = [
        "USDT", "USDC", "DAI", "BUSD", "TUSD", "FDUSD", "GUSD", "PYUSD", "USDD", "LUSD", "USDP",
        "FRAX", "USD1",
    ];
    stablecoins.contains(&symbol.to_ascii_uppercase().as_str())
}

async fn fetch_markets_for_exchange(
    exchange_name: &str,
    market_type: CexMarketType,
) -> Result<Vec<CexMarket>> {
    match exchange_name.to_ascii_lowercase().as_str() {
        "binance" => {
            let client = BinanceClient::new();
            match market_type {
                CexMarketType::Spot => {
                    let raw_spot = client
                        .get_exchange_info(BinanceMarketType::Spot)
                        .await
                        .context("binance spot exchangeInfo")?;
                    parse_binance_markets(&raw_spot, exchange_name).context("parse binance spot")
                }
                CexMarketType::Perps => {
                    let raw_perp = client
                        .get_exchange_info(BinanceMarketType::UsdFutures)
                        .await
                        .context("binance usd-m futures exchangeInfo")?;
                    parse_binance_markets(&raw_perp, exchange_name).context("parse binance perps")
                }
            }
        }

        "bybit" => {
            let client = BybitClient::new();
            match market_type {
                CexMarketType::Spot => {
                    let raw_spot = client
                        .get_instruments_info(BybitCategory::Spot, None)
                        .await
                        .context("bybit spot instruments")?;
                    parse_bybit_markets(&raw_spot, exchange_name, "SPOT")
                        .context("parse bybit spot")
                }
                CexMarketType::Perps => {
                    let raw_perp = client
                        .get_instruments_info(BybitCategory::Linear, None)
                        .await
                        .context("bybit linear instruments")?;
                    parse_bybit_markets(&raw_perp, exchange_name, "PERP")
                        .context("parse bybit perps")
                }
            }
        }

        other => Err(anyhow!("Market sync not supported for CEX: {other}")),
    }
}

pub async fn refresh_cex_markets(
    client: &Client,
    exchange_name: &str,
    selected_quote: Option<&str>,
    market_type: CexMarketType,
) -> Result<()> {
    if let Some(q) = selected_quote {
        let q_up = q.to_ascii_uppercase();
        if q_up != "USDT" && q_up != "USDC" {
            return Err(anyhow!("selected_quote must be \"USDT\" or \"USDC\""));
        }
    }

    match selected_quote {
        Some(q) => info!(
            "Refreshing markets for CEX: {} | type={:?} | quote={}",
            exchange_name,
            market_type,
            q.to_ascii_uppercase()
        ),
        None => info!(
            "Refreshing markets for CEX: {} | type={:?} | quotes=ALL",
            exchange_name, market_type
        ),
    }

    let exchange_id = upsert_cex_exchange(client, exchange_name).await?;

    let mut markets = fetch_markets_for_exchange(exchange_name, market_type).await?;

    markets.retain(|m| !is_stablecoin_symbol(&m.base_currency));

    if let Some(q) = selected_quote {
        markets.retain(|m| keep_selected_quote(m, q));
    }

    if markets.is_empty() {
        info!(
            "No markets after filtering: exchange={} type={:?} quote={:?}",
            exchange_name, market_type, selected_quote
        );
        return Ok(());
    }

    let count = markets.len();
    upsert_cex_markets(client, exchange_id, &markets).await?;
    info!(
        "Upserted {} markets for {} (id={} type={:?} quote={:?})",
        count, exchange_name, exchange_id, market_type, selected_quote
    );

    Ok(())
}
