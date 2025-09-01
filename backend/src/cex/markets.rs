use anyhow::{anyhow, Context, Result};
use sqlx::PgPool;
use tracing::info;

use crate::cex::common::CexMarketType;
use crate::exchanges::shared::types::CexMarket;
use crate::db::insert::{upsert_cex_exchange, upsert_cex_markets};

use crate::exchanges::binance::api::{client::BinanceClient, endpoints::MarketType as BinanceMarketType};
use crate::exchanges::bybit::api::{client::BybitClient, endpoints::Category as BybitCategory};
use crate::exchanges::binance::handler::handler::parse_binance_markets;
use crate::exchanges::bybit::handler::handler::parse_bybit_markets;

fn keep_selected_quote(m: &CexMarket, selected: &str) -> bool {
    let sel = selected.to_ascii_uppercase();
    let by_suffix = m.market_symbol.to_ascii_uppercase().ends_with(&sel);
    let by_quote  = m.quote_currency.to_ascii_uppercase() == sel;
    by_suffix || by_quote
}

fn keep_market_type(m: &CexMarket, mt: CexMarketType) -> bool {
    let t = m.market_type.to_ascii_lowercase();
    match mt {
        CexMarketType::Spot  => t == "spot",
        CexMarketType::Perps => {
            // be tolerant to different parsers (“perp”, “perps”, “perpetual”, “linear”, “usd-m”, “futures”)
            matches!(t.as_str(), "perp" | "perps" | "perpetual" | "linear" | "usd-m" | "futures")
        }
    }
}

/// Pull **all** markets from a given exchange (both spot + perps). We’ll filter later.
async fn fetch_markets_for_exchange(exchange_name: &str) -> Result<Vec<CexMarket>> {
    match exchange_name {
        "binance" => {
            let client = BinanceClient::new();

            // Spot
            let raw_spot = client
                .get_exchange_info(BinanceMarketType::Spot)
                .await
                .context("binance spot exchangeInfo")?;
            let mut out = parse_binance_markets(&raw_spot, exchange_name)
                .context("parse binance spot")?;

            // USD-M Perps
            let raw_perp = client
                .get_exchange_info(BinanceMarketType::UsdFutures)
                .await
                .context("binance usd-m futures exchangeInfo")?;
            let mut perps = parse_binance_markets(&raw_perp, exchange_name)
                .context("parse binance perps")?;

            out.append(&mut perps);
            Ok(out)
        }

        "bybit" => {
            let client = BybitClient::new();

            // Spot
            let raw_spot = client
                .get_instruments_info(BybitCategory::Spot, None)
                .await
                .context("bybit spot instruments")?;
            let mut out = parse_bybit_markets(&raw_spot, exchange_name, "SPOT")
                .context("parse bybit spot")?;

            // Linear Perps
            let raw_perp = client
                .get_instruments_info(BybitCategory::Linear, None)
                .await
                .context("bybit linear instruments")?;
            let mut perps = parse_bybit_markets(&raw_perp, exchange_name, "PERP")
                .context("parse bybit perps")?;

            out.append(&mut perps);
            Ok(out)
        }

        other => Err(anyhow!("Market sync not supported for CEX: {other}")),
    }
}

/// Refresh markets for one CEX:
/// - `market_type` selects **SPOT** or **PERPS** universe
/// - `selected_quote`: `None` = **ALL quotes**, `Some("USDT"/"USDC")` to restrict
pub async fn refresh_cex_markets(
    pool: &PgPool,
    exchange_name: &str,
    selected_quote: Option<&str>,   // None => ALL quotes
    market_type: CexMarketType,
) -> Result<()> {
    if let Some(q) = selected_quote {
        let q_up = q.to_ascii_uppercase();
        if q_up != "USDT" && q_up != "USDC" {
            // keep your previous safety check; relax here if you want FDUSD etc.
            return Err(anyhow!("selected_quote must be \"USDT\" or \"USDC\""));
        }
    }

    match selected_quote {
        Some(q) => info!("Refreshing markets for CEX: {} | type={:?} | quote={}", exchange_name, market_type, q.to_ascii_uppercase()),
        None    => info!("Refreshing markets for CEX: {} | type={:?} | quotes=ALL", exchange_name, market_type),
    }

    // upsert exchange row, get id
    let exchange_id = upsert_cex_exchange(pool, exchange_name).await?;

    // fetch all markets for this exchange
    let mut markets = fetch_markets_for_exchange(exchange_name).await?;

    // filter to requested market_type
    markets.retain(|m| keep_market_type(m, market_type));

    // optional quote filter
    if let Some(q) = selected_quote {
        markets.retain(|m| keep_selected_quote(m, q));
    }

    // nothing to do?
    if markets.is_empty() {
        info!("No markets after filtering: exchange={} type={:?} quote={:?}", exchange_name, market_type, selected_quote);
        return Ok(());
    }

    let count = markets.len();
    upsert_cex_markets(pool, exchange_id, &markets).await?;
    info!("Upserted {} markets for {} (id={} type={:?} quote={:?})", count, exchange_name, exchange_id, market_type, selected_quote);

    Ok(())
}
