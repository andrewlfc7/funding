#[derive(Debug, Clone, Copy)]
pub enum MarketType {
    Spot,
    UsdFutures,
}

// Base URLs
pub const SPOT_API_URL: &str = "https://api.binance.com";
pub const USD_FUTURES_API_URL: &str = "https://fapi.binance.com";

// Paths
pub const EXCHANGE_INFO_PATH: &str = "/api/v3/exchangeInfo";
pub const FUTURES_EXCHANGE_INFO_PATH: &str = "/fapi/v1/exchangeInfo";
pub const KLINES_PATH: &str = "/api/v3/klines";
pub const FUTURES_KLINES_PATH: &str = "/fapi/v1/klines";
pub const TRADES_PATH: &str = "/api/v3/trades";
pub const FUTURES_TRADES_PATH: &str = "/fapi/v1/trades";

