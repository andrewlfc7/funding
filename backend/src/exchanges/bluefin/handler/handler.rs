use anyhow::Result;
use bytes::Bytes;
use chrono::{LocalResult, TimeZone, Utc};

use crate::exchanges::bluefin::api::types::{
    BluefinExchangeInfoResponse, BluefinFundingRate, BluefinTicker,
};
use crate::exchanges::shared::types::{
    NormalizedFundingRate, NormalizedMarket, NormalizedMarketStats,
};

#[inline]
fn ts_utc(ms: i64) -> chrono::DateTime<Utc> {
    match Utc.timestamp_millis_opt(ms) {
        LocalResult::Single(dt) => dt,
        _ => Utc::now(),
    }
}

/// Parses the response from `get_exchange_info` into a list of normalized markets.
pub fn parse_bluefin_markets(raw: &Bytes) -> Result<Vec<NormalizedMarket>> {
    let resp: BluefinExchangeInfoResponse = serde_json::from_slice(raw)?;

    Ok(resp
        .markets
        .into_iter()
        .map(|m| NormalizedMarket {
            exchange: "bluefin".to_string(),
            symbol: m.base_asset_symbol.clone(),
            market_symbol: m.symbol,
            base_currency: m.base_asset_symbol,
            quote_currency: "USDC".to_string(),
            is_active: m.status.to_uppercase() == "ACTIVE",
        })
        .collect())
}

/// Parses the response from `get_tickers` into a list of market stats.
/// NOTE: This implementation stores the USD NOTIONAL open interest, as requested.
pub fn parse_bluefin_market_stats(raw: &Bytes) -> Result<Vec<NormalizedMarketStats>> {
    let resp: Vec<BluefinTicker> = serde_json::from_slice(raw)?;

    Ok(resp
        .into_iter()
        .map(|ticker| NormalizedMarketStats {
            market_symbol: ticker.symbol,
            open_interest: Some(ticker.open_interest_e9),
            volume_24h: Some(ticker.quote_volume_24hr_e9),
            timestamp: ts_utc(ticker.updated_at_millis),
        })
        .collect())
}

/// Parses the response from `get_funding_rate_history` into a list of normalized funding rates.
pub fn parse_bluefin_funding(raw: &Bytes) -> Result<Vec<NormalizedFundingRate>> {
    let resp: Vec<BluefinFundingRate> = serde_json::from_slice(raw)?;

    Ok(resp
        .into_iter()
        .map(|f| NormalizedFundingRate {
            market_symbol: f.symbol,
            rate: f.funding_rate_e9,
            timestamp: ts_utc(f.funding_time_at_millis),
        })
        .collect())
}
