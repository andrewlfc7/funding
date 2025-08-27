use serde::Deserialize;
use rust_decimal::Decimal;

/// GET /info/markets
#[derive(Debug, Deserialize)]
pub struct ExtendedMarketsResponse {
    pub data: Vec<ExtendedMarket>,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedMarket {
    pub name: String,        // "BTC-USD"
    pub assetName: String,   // "BTC"
    pub active: bool,
    pub marketStats: Option<ExtendedInlineMarketStats>,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedInlineMarketStats {
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub openInterest: Decimal,
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub dailyVolume: Decimal,
}

/// GET /funding
#[derive(Debug, Deserialize)]
pub struct ExtendedFundingResponse {
    pub data: Vec<ExtendedFundingData>,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedFundingData {
    pub m: String, // market symbol
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub f: Decimal, // funding rate
    pub T: i64,     // timestamp (ms)
}

/// GET /markets/{symbol}/stats
#[derive(Debug, Deserialize)]
pub struct ExtendedMarketStatsResponse {
    pub data: ExtendedMarketStatsData,
}

#[derive(Debug, Deserialize)]
pub struct ExtendedMarketStatsData {
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub openInterest: Decimal,
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub dailyVolume: Decimal,
}

fn deserialize_decimal_from_str<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}