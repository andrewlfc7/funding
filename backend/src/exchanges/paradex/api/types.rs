use serde::{Deserialize, Deserializer};
use rust_decimal::Decimal;

/* Flexible decimal parsing: can be number, string, or null */
#[derive(Deserialize)]
#[serde(untagged)]
enum NumOrStrDec {
    N(Decimal),
    S(String),
}

pub fn de_decimal<'de, D>(d: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    match NumOrStrDec::deserialize(d)? {
        NumOrStrDec::N(v) => Ok(v),
        NumOrStrDec::S(s) => s.parse().map_err(serde::de::Error::custom),
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum OptNumOrStrDec {
    Null,
    N(Decimal),
    S(String),
}

pub fn de_opt_decimal<'de, D>(d: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(match OptNumOrStrDec::deserialize(d)? {
        OptNumOrStrDec::Null => None,
        OptNumOrStrDec::N(v) => Some(v),
        OptNumOrStrDec::S(s) => {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                Some(s.parse().map_err(serde::de::Error::custom)?)
            }
        }
    })
}

/* Timestamp can also be number or string */
#[derive(Deserialize)]
#[serde(untagged)]
enum I64OrStr {
    I(i64),
    S(String),
}

pub fn de_ms_i64<'de, D>(d: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    match I64OrStr::deserialize(d)? {
        I64OrStr::I(v) => Ok(v),
        I64OrStr::S(s) => s.parse::<i64>().map_err(serde::de::Error::custom),
    }
}

/* ========== API Response Models ========== */

#[derive(Debug, Deserialize)]
pub struct ParadexMarketsResponse {
    pub results: Vec<ParadexMarket>,
}

#[derive(Debug, Deserialize)]
pub struct ParadexMarket {
    pub symbol: String,
    pub base_currency: String,
    pub quote_currency: String,
    #[serde(default)]
    pub asset_kind: Option<String>,
}

/* /markets/summary */
#[derive(Debug, Deserialize)]
pub struct ParadexSummaryResponse {
    pub results: Vec<ParadexMarketSummary>,
}

#[derive(Debug, Deserialize)]
pub struct ParadexMarketSummary {
    pub symbol: String,

    #[serde(deserialize_with = "de_decimal")]
    pub open_interest: Decimal,

    #[serde(deserialize_with = "de_decimal")]
    pub volume_24h: Decimal,
}

/* /funding */
#[derive(Debug, Deserialize)]
pub struct ParadexFunding {
    pub market: String,

    #[serde(deserialize_with = "de_opt_decimal")]
    pub funding_index: Option<Decimal>,

    #[serde(deserialize_with = "de_opt_decimal")]
    pub funding_premium: Option<Decimal>,

    #[serde(deserialize_with = "de_opt_decimal")]
    pub funding_rate: Option<Decimal>,

    #[serde(deserialize_with = "de_ms_i64")]
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct ParadexFundingResponse {
    pub next: Option<String>,
    pub prev: Option<String>,
    pub results: Vec<ParadexFunding>,
}