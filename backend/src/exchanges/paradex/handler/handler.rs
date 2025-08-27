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

/* -------- funding -------- */

pub fn parse_paradex_funding(raw: &Bytes) -> Result<Vec<NormalizedFundingRate>> {
    let (rows, _next) = parse_paradex_funding_page(raw)?;
    Ok(rows)
}
pub fn parse_paradex_funding_page(
    raw: &Bytes,
) -> Result<(Vec<NormalizedFundingRate>, Option<String>)> {
    let text = std::str::from_utf8(raw)
        .with_context(|| "Failed to decode Paradex response as UTF-8")?;

    let resp: ParadexFundingResponse = serde_json::from_str(text)
        .with_context(|| format!("Failed to parse ParadexFundingResponse JSON. Input text: '{}'", text))?;

    let rows = resp
        .results
        .into_iter()
        .filter_map(|f| {
            f.funding_rate.map(|rate| NormalizedFundingRate {
                market_symbol: f.market,
                rate,
                timestamp: ts_utc(f.created_at),
            })
        })
        .collect();

    Ok((rows, resp.next))
}
/* -------- market stats -------- */

pub fn parse_paradex_market_stats(raw: &Bytes) -> Result<Vec<NormalizedMarketStats>> {
    let resp: ParadexSummaryResponse = serde_json::from_slice(raw)?;
    Ok(resp
        .results
        .into_iter()
        .filter(|s| perp_symbol(&s.symbol))
        .map(|s| NormalizedMarketStats {
            market_symbol: s.symbol,
            open_interest: Some(s.open_interest),
            volume_24h: Some(s.volume_24h),
            timestamp: Utc::now(),
        })
        .collect())
}