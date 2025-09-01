use anyhow::Result;
use bytes::Bytes;
use chrono::{LocalResult, TimeZone, Utc};
use rust_decimal::Decimal;

use crate::exchanges::drift::api::types::{
    DriftContractsResponse, DriftFundingRatesResponse
};
use crate::exchanges::shared::types::{NormalizedFundingRate, NormalizedMarket, NormalizedMarketStats};


#[inline]
fn ts_utc_seconds_str(s: &str) -> chrono::DateTime<Utc> {
    s.parse::<i64>()
        .ok()
        .and_then(|ts_secs| Utc.timestamp_opt(ts_secs, 0).single())
        .unwrap_or_else(Utc::now)
}

pub fn parse_drift_markets(raw: &Bytes) -> Result<Vec<NormalizedMarket>> {
    let resp: DriftContractsResponse = serde_json::from_slice(raw)?;

    Ok(resp
        .contracts
        .into_iter()
        .filter(|c| c.ticker_id.ends_with("-PERP"))
        .map(|c| NormalizedMarket {
            exchange: "drift".to_string(),
            symbol: c.base_currency.clone(),
            market_symbol: c.ticker_id,
            base_currency: c.base_currency,
            quote_currency: c.quote_currency,
            is_active: true,
        })
        .collect())
}


pub fn parse_drift_market_stats(raw: &Bytes) -> Result<Vec<NormalizedMarketStats>> {
    let resp: DriftContractsResponse = serde_json::from_slice(raw)?;
    let now = Utc::now();

    Ok(resp
        .contracts
        .into_iter()
        .filter(|c| c.ticker_id.ends_with("-PERP"))
        .map(|c| {
            // Safely calculate the notional open interest.
            // This will only produce a value if BOTH open_interest AND index_price are present.
            let open_interest_notional = if let (Some(oi_coins), Some(price)) = (c.open_interest, c.index_price) {
                Some(oi_coins * price)
            } else {
                None // If either is missing, the result is None
            };

            NormalizedMarketStats {
                market_symbol: c.ticker_id,
                open_interest: open_interest_notional, 
                volume_24h: c.quote_volume,
                timestamp: now,
            }
        })
        .collect())
}


pub fn parse_drift_funding(raw: &Bytes, market_symbol: &str) -> Result<Vec<NormalizedFundingRate>> {
    let resp: DriftFundingRatesResponse = serde_json::from_slice(raw)?;

    Ok(resp
        .funding_rates
        .into_iter()
        .map(|f| {
            let final_rate = if f.oracle_price_twap > Decimal::ZERO {
                f.funding_rate / f.oracle_price_twap
            } else {
                Decimal::ZERO 
            };

            NormalizedFundingRate {
                market_symbol: market_symbol.to_string(),
                rate: final_rate,
                timestamp: ts_utc_seconds_str(&f.ts),
            }
        })
        .collect())
}