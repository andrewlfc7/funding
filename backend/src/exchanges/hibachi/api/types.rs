use rust_decimal::Decimal;
use serde::Deserialize;

// Helper function to deserialize numeric strings into Decimal.
// It's good practice to keep this available for all exchange-specific type modules.
pub fn deserialize_decimal_from_str<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

// ----- For `get_exchange_info` -> /market/exchange-info -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HibachiExchangeInfoResponse {
    pub future_contracts: Vec<HibachiFutureContract>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HibachiFutureContract {
    pub symbol: String,           // e.g., "ETH/USDT-P"
    pub underlying_symbol: String, // e.g., "ETH"
    pub settlement_symbol: String, // e.g., "USDT"
    pub status: String,           // e.g., "LIVE"
}

// ----- For `get_open_interest` -> /market/data/open-interest -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HibachiOpenInterestResponse {
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub total_quantity: Decimal, // This is the open interest in coin units
}

// ----- For `get_stats` -> /market/data/stats -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HibachiStatsResponse {
    pub symbol: String,
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub volume_24h: Decimal,
}

// ----- For `get_prices` -> /market/data/prices -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HibachiPricesResponse {
    pub symbol: String,
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub mark_price: Decimal,
    // Add this field to capture the nested funding object
    pub funding_rate_estimation: HibachiFundingRateEstimation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HibachiFundingRateEstimation {
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub estimated_funding_rate: Decimal,
    pub next_funding_timestamp: i64, // This is a Unix timestamp in seconds
}