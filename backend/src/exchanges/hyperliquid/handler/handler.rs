use anyhow::Result;
use bytes::Bytes;
use chrono::{LocalResult, TimeZone, Utc};

use crate::exchanges::hyperliquid::api::types::{
    HyperliquidAssetCtx, HyperliquidFundingHistoryEntry, HyperliquidMetaResponse,
    HyperliquidUniverseWrapper,
};
use crate::exchanges::shared::types::{NormalizedFundingRate, NormalizedMarket, NormalizedMarketStats};

#[inline]
fn ts_utc(ms: i64) -> chrono::DateTime<Utc> {
    match Utc.timestamp_millis_opt(ms) {
        LocalResult::Single(dt) => dt,
        _ => Utc::now(), // Fallback for invalid timestamp
    }
}

pub fn parse_hyperliquid_markets(raw: &Bytes) -> Result<Vec<NormalizedMarket>> {
    let resp: HyperliquidMetaResponse = serde_json::from_slice(raw)?;

    Ok(resp
        .universe
        .into_iter()
        .map(|m| NormalizedMarket {
            exchange: "hyperliquid".to_string(),
            symbol: m.name.clone(),
            market_symbol: m.name.clone(), 
            base_currency: m.name,
            quote_currency: "USD".to_string(), // Hyperliquid perps are quoted in USD
            is_active: !m.is_delisted,
        })
        .collect())
}


pub fn parse_hyperliquid_market_stats(raw: &Bytes) -> Result<Vec<NormalizedMarketStats>> {
    let (universe_wrapper, ctxs): (HyperliquidUniverseWrapper, Vec<HyperliquidAssetCtx>) =
        serde_json::from_slice(raw)?;

    let now = Utc::now(); 
    
    Ok(universe_wrapper
        .universe
        .into_iter()
        .zip(ctxs.into_iter())
        .map(|(market_info, stats)| {

            let open_interest_usd = stats.open_interest * stats.mark_px;

            NormalizedMarketStats {
                market_symbol: market_info.name,
                open_interest: Some(open_interest_usd), 
                volume_24h: Some(stats.day_ntl_vlm), 
                timestamp: now,
            }
        })
        .collect())
}


pub fn parse_hyperliquid_funding(raw: &Bytes) -> Result<Vec<NormalizedFundingRate>> {
    let resp: Vec<HyperliquidFundingHistoryEntry> = serde_json::from_slice(raw)?;

    Ok(resp
        .into_iter()
        .map(|f| NormalizedFundingRate {
            market_symbol: f.coin,
            rate: f.funding_rate,
            timestamp: ts_utc(f.time),
        })
        .collect())
}





