use anyhow::Result;
use bytes::Bytes;
use chrono::{Duration, TimeZone, Utc}; // Add Duration and TimeZone

use crate::exchanges::hibachi::api::types::{
    HibachiExchangeInfoResponse, HibachiOpenInterestResponse, HibachiPricesResponse, HibachiStatsResponse,
};
use crate::exchanges::shared::types::{NormalizedMarket, NormalizedMarketStats, NormalizedFundingRate};
use anyhow::anyhow;


/// Parses the response from `get_exchange_info` into a list of normalized markets.
pub fn parse_hibachi_markets(raw: &Bytes) -> Result<Vec<NormalizedMarket>> {
    let resp: HibachiExchangeInfoResponse = serde_json::from_slice(raw)?;

    Ok(resp
        .future_contracts
        .into_iter()
        .map(|c| NormalizedMarket {
            exchange: "hibachi".to_string(),
            symbol: c.underlying_symbol.clone(),
            market_symbol: c.symbol,
            base_currency: c.underlying_symbol,
            quote_currency: c.settlement_symbol,
            is_active: c.status.to_uppercase() == "LIVE",
        })
        .collect())
}


pub fn parse_hibachi_market_stats(
    raw_open_interest: &Bytes,
    raw_stats: &Bytes,
    raw_prices: &Bytes,
    market_symbol: &str, // Pass in the symbol for context
) -> Result<NormalizedMarketStats> {
    let oi_resp: HibachiOpenInterestResponse = serde_json::from_slice(raw_open_interest)?;
    let stats_resp: HibachiStatsResponse = serde_json::from_slice(raw_stats)?;
    let prices_resp: HibachiPricesResponse = serde_json::from_slice(raw_prices)?;

    let open_interest_usd = oi_resp.total_quantity * prices_resp.mark_price;
    Ok(NormalizedMarketStats {
        market_symbol: market_symbol.to_string(),
        open_interest: Some(oi_resp.total_quantity),
        volume_24h: Some(stats_resp.volume_24h),
        timestamp: Utc::now(),
    })
}

pub fn parse_hibachi_latest_funding(raw_prices: &Bytes) -> Result<NormalizedFundingRate> {
    let resp: HibachiPricesResponse = serde_json::from_slice(raw_prices)?;

    let funding_info = resp.funding_rate_estimation;


    let next_funding_time = Utc.timestamp_opt(funding_info.next_funding_timestamp, 0)
        .single()
        .ok_or_else(|| anyhow!("Invalid next_funding_timestamp from API: {}", funding_info.next_funding_timestamp))?;

    // The timestamp for our normalized model represents the start of the period.
    let current_period_start_time = next_funding_time - Duration::hours(8);

    Ok(NormalizedFundingRate {
        market_symbol: resp.symbol,
        rate: funding_info.estimated_funding_rate,
        timestamp: current_period_start_time,
    })
}