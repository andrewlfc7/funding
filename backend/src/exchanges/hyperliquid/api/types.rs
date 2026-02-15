use rust_decimal::Decimal;
use serde::Deserialize;

// Helper function to deserialize numeric strings into Decimal.
pub fn deserialize_decimal_from_str<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

// ----- For `get_perp_meta` (type: "meta") -----

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidMetaResponse {
    pub universe: Vec<HyperliquidUniverseEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidUniverseEntry {
    pub name: String,
    #[serde(default)] // isDelisted is not always present
    pub is_delisted: bool,
}

// ----- For `get_meta_and_asset_ctxs` (type: "metaAndAssetCtxs") -----
// The response is a tuple: [ { "universe": [...] }, [ { "dayNtlVlm": ... } ] ]

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidUniverseWrapper {
    pub universe: Vec<HyperliquidUniverseEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidAssetCtx {
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub day_ntl_vlm: Decimal, // 24h Volume (already in USD)

    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub open_interest: Decimal, // In number of contracts/coins

    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub mark_px: Decimal, // The Mark Price, used for valuation
}

// ----- For `get_funding_history` (type: "fundingHistory") -----
// The response is a direct array of these entries.

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperliquidFundingHistoryEntry {
    pub coin: String,
    #[serde(deserialize_with = "deserialize_decimal_from_str")]
    pub funding_rate: Decimal,
    pub time: i64, // Millisecond timestamp
}
