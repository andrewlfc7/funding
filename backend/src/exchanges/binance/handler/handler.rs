use anyhow::Result;
use bytes::Bytes;
use chrono::{TimeZone, Utc, LocalResult};

use crate::exchanges::shared::types::{NormalizedKline,CexMarket, NormalizedTrade};
use crate::exchanges::binance::api::types::{BinanceTrade,BinanceExchangeInfoResponse};
use anyhow::Context;
use crate::utils::utils::is_usd_stable;


pub fn parse_binance_markets(raw_data: &[u8], exchange_name: &str) -> Result<Vec<CexMarket>> {
    #[derive(serde::Deserialize)]
    struct BinanceSymbol {
        symbol: String,
        status: String,
        baseAsset: String,
        quoteAsset: String,
        #[serde(default)]
        contractType: Option<String>, // "PERPETUAL" for USD-M futs; None for spot
    }
    #[derive(serde::Deserialize)]
    struct BinanceExchangeInfoResponse {
        symbols: Vec<BinanceSymbol>,
    }

    let response: BinanceExchangeInfoResponse =
        serde_json::from_slice(raw_data).context("Failed to parse Binance exchange info")?;

    let markets = response
        .symbols
        .into_iter()
        .filter(|s| s.status == "TRADING")
        // keep only spot (no contractType) or USD-M perpetuals (PERPETUAL)
        .filter(|s| s.contractType.as_deref() == Some("PERPETUAL") || s.contractType.is_none())
        // keep only USDT/USDC quote
        .filter(|s| is_usd_stable(&s.quoteAsset))
        .map(|s| {
            let market_type_db = if s.contractType.is_some() { "perps" } else { "spot" };
            CexMarket {
                exchange: exchange_name.to_string(),
                symbol: s.baseAsset.clone(),       // base only, keep case
                market_symbol: s.symbol.clone(),   // e.g. "BTCUSDT"
                base_currency: s.baseAsset,
                quote_currency: s.quoteAsset,      // USDT/USDC only due to filter
                market_type: market_type_db.to_string(),
                is_active: true,
            }
        })
        .collect();

    Ok(markets)
}


pub fn parse_binance_klines(raw_data: &[u8], market_symbol: &str) -> Result<Vec<NormalizedKline>> {
    let response: Vec<Vec<serde_json::Value>> = serde_json::from_slice(raw_data)
        .context("Failed to parse Binance klines")?;

    response.into_iter().map(|kline| {
        Ok(NormalizedKline {
            market_symbol: market_symbol.to_string(),
            open_time: Utc.timestamp_millis_opt(kline[0].as_i64().context("Invalid open_time")?).unwrap(),
            open: kline[1].as_str().context("Invalid open price")?.parse()?,
            high: kline[2].as_str().context("Invalid high price")?.parse()?,
            low: kline[3].as_str().context("Invalid low price")?.parse()?,
            close: kline[4].as_str().context("Invalid close price")?.parse()?,
            volume: kline[5].as_str().context("Invalid volume")?.parse()?,
        })
    }).collect()
}

pub fn parse_binance_trades(raw_data: &[u8], market_symbol: &str) -> Result<Vec<NormalizedTrade>> {
    // If top-level is an error object, surface it clearly (no extra deps)
    if raw_data.first() == Some(&b'{') {
        #[derive(serde::Deserialize)]
        struct ErrObj { code: Option<i64>, msg: Option<String> }
        if let Ok(e) = serde_json::from_slice::<ErrObj>(raw_data) {
            if e.code.is_some() || e.msg.is_some() {
                return Err(anyhow::anyhow!(
                    "Binance API error: code={:?} msg={:?}",
                    e.code, e.msg
                ));
            }
        }
    }

    let response: Vec<BinanceTrade> = serde_json::from_slice(raw_data)
        .context("Failed to parse Binance trades (expected /api/v3/trades or /api/v3/aggTrades)")?;

    response
        .into_iter()
        .map(|trade| {
            let side = if trade.isBuyerMaker { "Sell" } else { "Buy" };
            let ts = match Utc.timestamp_millis_opt(trade.time) {
                LocalResult::Single(dt) => dt,
                _ => Utc.timestamp_millis(0),
            };

            Ok(NormalizedTrade {
                market_symbol: market_symbol.to_string(),
                trade_id: trade.id.to_string(),
                trade_time: ts,
                side: side.to_string(),
                price: trade.price.parse()?,
                qty: trade.qty.parse()?,
                quote_qty: trade.quoteQty.as_deref().unwrap_or("0").parse()?, // <= tolerant
            })
        })
        .collect()
}
