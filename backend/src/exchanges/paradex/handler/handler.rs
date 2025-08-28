use rust_decimal::prelude::ToPrimitive;
use anyhow::{Context, Result};
use bytes::Bytes;
use chrono::{LocalResult, TimeZone, Utc};
use rust_decimal::Decimal;

use crate::exchanges::shared::types::{
    NormalizedFundingRate, NormalizedMarket, NormalizedMarketStats,
};
use crate::exchanges::paradex::api::types::{
    ParadexFundingResponse, ParadexMarketsResponse, ParadexSummaryResponse,
};

#[inline]
fn ts_utc(ms: i64) -> chrono::DateTime<Utc> {
    match Utc.timestamp_millis_opt(ms) {
        LocalResult::Single(dt) => dt,
        _ => Utc::now(),
    }
}

#[inline]
fn perp_symbol(sym: &str) -> bool {
    let s = sym.to_ascii_uppercase();
    s.ends_with("-PERP") || s.ends_with("-PERPS")
}

/* -------- markets -------- */

pub fn parse_paradex_markets(raw: &Bytes) -> Result<Vec<NormalizedMarket>> {
    let resp: ParadexMarketsResponse = serde_json::from_slice(raw)?;
    Ok(resp
        .results
        .into_iter()
        .filter(|m| perp_symbol(&m.symbol))
        .map(|m| NormalizedMarket {
            exchange: "paradex".to_string(),
            symbol: m.base_currency.clone(),
            market_symbol: m.symbol,
            base_currency: m.base_currency,
            quote_currency: m.quote_currency,
            is_active: true,
        })
        .collect())
}



pub fn parse_paradex_market_stats(raw: &Bytes) -> Result<Vec<NormalizedMarketStats>> {
    let resp: ParadexSummaryResponse = serde_json::from_slice(raw)?;
    let now = Utc::now();

    let out = resp
        .results
        .into_iter()
        .filter(|s| perp_symbol(&s.symbol))
        .map(|s| {
            // choose a USD price: mark → underlying → last_traded
            let px = s
                .mark_price
                .or(s.underlying_price)
                .or(s.last_traded_price);

            // OI is in base units; convert to USD if we have a price
            let oi_usd = match (s.open_interest, px) {
                (Some(oi_base), Some(price)) => Some(oi_base * price),
                // If no price, leave OI unset so the UI doesn't mix units.
                _ => None,
            };

            NormalizedMarketStats {
                market_symbol: s.symbol,
                open_interest: oi_usd,
                // volume_24h is already USD per API; keep as-is if present
                volume_24h: s.volume_24h,
                timestamp: now, // summary doesn’t carry a per-row ts
            }
        })
        .collect();

    Ok(out)
}



pub fn parse_paradex_funding(raw: &bytes::Bytes) -> anyhow::Result<Vec<crate::exchanges::shared::types::NormalizedFundingRate>> {
    use anyhow::Context;
    use crate::exchanges::paradex::api::types::ParadexFundingResponse;

    let text = std::str::from_utf8(raw).context("decode Paradex funding UTF-8")?;
    let resp: ParadexFundingResponse = serde_json::from_str(text).context("parse ParadexFundingResponse")?;

    Ok(resp
        .results
        .into_iter()
        .filter_map(|f| {
            f.funding_rate.map(|rate| crate::exchanges::shared::types::NormalizedFundingRate {
                market_symbol: f.market,
                rate,                              // already a Decimal fraction (raw per-interval)
                timestamp: ts_utc(f.created_at), // ms -> DateTime<Utc>
            })
        })
        .collect())
}
