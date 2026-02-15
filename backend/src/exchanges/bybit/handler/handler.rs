use crate::exchanges::bybit::api::types::BybitKlineResponse;
use crate::exchanges::bybit::api::types::BybitTradeResponse;
use crate::exchanges::shared::types::{CexMarket, NormalizedKline, NormalizedTrade};
use crate::utils::utils::is_usd_stable;
use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;

pub fn parse_bybit_markets(
    raw_data: &[u8],
    exchange_name: &str,
    market_category: &str, // "SPOT" | "PERP"
) -> Result<Vec<CexMarket>> {
    #[derive(serde::Deserialize)]
    struct BybitListItem {
        symbol: String,
        status: String, // "Trading"
        baseCoin: String,
        quoteCoin: String, // we'll filter to USDT/USDC only
    }
    #[derive(serde::Deserialize)]
    struct BybitResult {
        list: Vec<BybitListItem>,
    }
    #[derive(serde::Deserialize)]
    struct BybitInstrumentsResponse {
        result: BybitResult,
    }

    let response: BybitInstrumentsResponse =
        serde_json::from_slice(raw_data).context("Failed to parse Bybit instruments response")?;

    let market_type_db = if market_category.eq_ignore_ascii_case("PERP") {
        "perps"
    } else {
        "spot"
    };

    let markets = response
        .result
        .list
        .into_iter()
        .filter(|s| s.status == "Trading")
        .filter(|s| is_usd_stable(&s.quoteCoin)) // USDT/USDC only
        .map(|s| CexMarket {
            exchange: exchange_name.to_string(),
            symbol: s.baseCoin.clone(),
            market_symbol: s.symbol.clone(),
            base_currency: s.baseCoin,
            quote_currency: s.quoteCoin, // USDT/USDC only due to filter
            market_type: market_type_db.to_string(),
            is_active: true,
        })
        .collect();

    Ok(markets)
}

pub fn parse_bybit_klines(raw_data: &[u8], market_symbol: &str) -> Result<Vec<NormalizedKline>> {
    let response: BybitKlineResponse =
        serde_json::from_slice(raw_data).context("Failed to parse Bybit klines")?;

    response
        .result
        .list
        .into_iter()
        .map(|kline| {
            Ok(NormalizedKline {
                market_symbol: market_symbol.to_string(),
                open_time: Utc.timestamp_millis_opt(kline[0].parse()?).unwrap(),
                open: kline[1].parse()?,
                high: kline[2].parse()?,
                low: kline[3].parse()?,
                close: kline[4].parse()?,
                volume: kline[5].parse()?,
            })
        })
        .collect()
}

// --- Parser for Trades ---
pub fn parse_bybit_trades(raw_data: &[u8], market_symbol: &str) -> Result<Vec<NormalizedTrade>> {
    let response: BybitTradeResponse =
        serde_json::from_slice(raw_data).context("Failed to parse Bybit trades")?;

    response
        .result
        .list
        .into_iter()
        .map(|trade| {
            let price: Decimal = trade.price.parse()?;
            let qty: Decimal = trade.size.parse()?;
            let quote_qty = price * qty; // Calculate quote_qty

            Ok(NormalizedTrade {
                market_symbol: market_symbol.to_string(),
                trade_id: trade.execId,
                trade_time: Utc.timestamp_millis_opt(trade.time.parse()?).unwrap(),
                side: trade.side,
                price,
                qty,
                quote_qty,
            })
        })
        .collect()
}
