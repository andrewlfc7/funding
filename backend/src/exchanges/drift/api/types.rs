use rust_decimal::Decimal;
use serde::Deserialize;

/// Deserializes a string that may be null or invalid into an Option<Decimal>.
pub fn deserialize_decimal_from_str_opt<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt_s: Option<&str> = Option::deserialize(deserializer)?;
    match opt_s {
        Some(s) => Ok(s.parse::<Decimal>().ok()),
        None => Ok(None),
    }
}

/// Deserializes Drift's funding rate premium (a large integer string scaled by 10^9) into a Decimal.
pub fn deserialize_funding_rate_from_str<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let val = s.parse::<Decimal>().map_err(serde::de::Error::custom)?;
    // This is the "premium", scaled by 1e9
    Ok(val / Decimal::new(1_000_000_000, 0))
}

/// Deserializes Drift's TWAP price (an integer string scaled by 10^6) into a Decimal.
pub fn deserialize_price_from_str<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let val = s.parse::<Decimal>().map_err(serde::de::Error::custom)?;
    // The oracle TWAP is scaled by 1e6
    Ok(val / Decimal::new(1_000_000, 0))
}

// ----- For `get_contracts` -> /contracts -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftContractsResponse {
    pub contracts: Vec<DriftContract>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DriftContract {
    pub ticker_id: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub product_type: String,
    
    #[serde(deserialize_with = "deserialize_decimal_from_str_opt")]
    pub open_interest: Option<Decimal>,
    #[serde(deserialize_with = "deserialize_decimal_from_str_opt")]
    pub quote_volume: Option<Decimal>,
    #[serde(deserialize_with = "deserialize_decimal_from_str_opt")]
    pub index_price: Option<Decimal>,
}

// ----- For `get_funding_rates` -> /fundingRates -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftFundingRatesResponse {
    pub funding_rates: Vec<DriftFundingRate>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriftFundingRate {
    pub ts: String, // Unix timestamp in seconds as a string
    // This is the premium value
    #[serde(deserialize_with = "deserialize_funding_rate_from_str")]
    pub funding_rate: Decimal,
    // This is the price used for normalization
    #[serde(deserialize_with = "deserialize_price_from_str")]
    pub oracle_price_twap: Decimal,
}