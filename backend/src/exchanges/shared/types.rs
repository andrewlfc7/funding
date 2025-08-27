use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMarket {
    pub exchange: String,
    pub symbol: String,
    pub market_symbol: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedFundingRate {
    pub market_symbol: String,
    pub rate: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMarketStats {
    pub market_symbol: String,
    pub open_interest: Option<Decimal>,
    pub volume_24h: Option<Decimal>,
    pub timestamp: DateTime<Utc>,
}
