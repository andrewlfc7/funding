use anyhow::Result;
use bytes::Bytes;
use chrono::{TimeZone, Utc, LocalResult};
use crate::exchanges::shared::types::{NormalizedFundingRate, NormalizedMarketStats, NormalizedMarket};
use crate::exchanges::extended::api::types::{ExtendedFundingResponse, ExtendedMarketsResponse, ExtendedMarketStatsResponse};

#[inline]
fn ts_utc(ms: i64) -> chrono::DateTime<Utc> {
    match Utc.timestamp_millis_opt(ms) {
        LocalResult::Single(dt) => dt,
        _ => Utc::now(),
    }
}

/// Parse /info/markets
pub fn parse_extended_markets(raw: &Bytes) -> Result<Vec<NormalizedMarket>> {
    let resp: ExtendedMarketsResponse = serde_json::from_slice(raw)?;
    Ok(resp.data.into_iter().map(|m| NormalizedMarket {
        exchange: "extended".to_string(),
        symbol: m.assetName.to_string(),
        market_symbol: m.name.to_string(),
        base_currency: m.assetName.to_string(),
        quote_currency: m.name.split('-').nth(1).unwrap_or("").to_string(),
        is_active: m.active,
    }).collect())
}

/// Parse /funding
pub fn parse_extended_funding(raw: &Bytes) -> Result<Vec<NormalizedFundingRate>> {
    let resp: ExtendedFundingResponse = serde_json::from_slice(raw)?;
    Ok(resp.data.into_iter().map(|f| NormalizedFundingRate {
        market_symbol: f.m.to_string(),
        rate: f.f,
        timestamp: ts_utc(f.T),
    }).collect())
}

/// Parse /markets/{symbol}/stats
pub fn parse_extended_market_stats(raw: &Bytes, market: &str) -> Result<NormalizedMarketStats> {
    let resp: ExtendedMarketStatsResponse = serde_json::from_slice(raw)?;
    Ok(NormalizedMarketStats {
        market_symbol: market.to_string(),
        open_interest: Some(resp.data.openInterest),
        volume_24h: Some(resp.data.dailyVolume), 
        timestamp: Utc::now(),
    })
}
