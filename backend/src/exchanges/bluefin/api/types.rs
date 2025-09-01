use rust_decimal::Decimal;
use serde::Deserialize;

/// Deserializes a string-encoded number scaled by 10^9 into a Decimal.
pub fn deserialize_decimal_from_e9_str<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let val = s.parse::<Decimal>().map_err(serde::de::Error::custom)?;
    // Divide by 10^9 to get the real value
    Ok(val / Decimal::new(1_000_000_000, 0))
}

// ----- For `get_exchange_info` -> /v1/exchange/info -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluefinExchangeInfoResponse {
    pub markets: Vec<BluefinMarket>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluefinMarket {
    pub symbol: String,
    pub base_asset_symbol: String,
    pub status: String,
}

// ----- For `get_tickers` -> /v1/exchange/tickers -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluefinTicker {
    pub symbol: String,
    
    #[serde(deserialize_with = "deserialize_decimal_from_e9_str")]
    pub mark_price_e9: Decimal,
    
    #[serde(deserialize_with = "deserialize_decimal_from_e9_str")]
    pub open_interest_e9: Decimal, // This is the NOTIONAL value (USD)
    
    #[serde(deserialize_with = "deserialize_decimal_from_e9_str")]
    pub quote_volume_24hr_e9: Decimal,
    
    pub updated_at_millis: i64,
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BluefinFundingRate {
    pub symbol: String,
    
    #[serde(deserialize_with = "deserialize_decimal_from_e9_str")]
    pub funding_rate_e9: Decimal,
    
    pub funding_time_at_millis: i64,
}