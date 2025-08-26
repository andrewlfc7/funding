// src/types/paradex.rs
use serde::Deserialize;
use rust_decimal::Decimal;

// Markets endpoint response
#[derive(Debug, Deserialize)]
pub struct ParadexMarketsResponse<'a> {
    #[serde(borrow)]
    pub results: Vec<ParadexMarket<'a>>,
}

#[derive(Debug, Deserialize)]
pub struct ParadexMarket<'a> {
    #[serde(borrow)]
    pub symbol: &'a str,
    #[serde(borrow)]
    pub base_currency: &'a str,
    #[serde(borrow)]
    pub quote_currency: &'a str,
}

// Market summary endpoint response
#[derive(Debug, Deserialize)]
pub struct ParadexSummaryResponse<'a> {
    #[serde(borrow)]
    pub results: Vec<ParadexMarketSummary<'a>>,
}

#[derive(Debug, Deserialize)]
pub struct ParadexMarketSummary<'a> {
    #[serde(borrow)]
    pub symbol: &'a str,
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub funding_rate: Decimal,
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub open_interest: Decimal,
}

// Funding data endpoint response
#[derive(Debug, Deserialize)]
pub struct ParadexFundingResponse<'a> {
    #[serde(borrow)]
    pub results: Vec<ParadexFundingData<'a>>,
    pub next: Option<&'a str>,
    pub prev: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct ParadexFundingData<'a> {
    #[serde(borrow)]
    pub market: &'a str,
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub funding_rate: Decimal,
    pub created_at: i64,
}

// Helper for deserializing string decimals
fn deserialize_decimal_from_str<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}