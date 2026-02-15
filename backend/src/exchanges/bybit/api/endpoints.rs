#[derive(Debug, Clone, Copy)]
pub enum Category {
    Spot,
    Linear,  // USDT & USDC Perps
    Inverse, // COIN-M Perps
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Spot => "spot",
            Category::Linear => "linear",
            Category::Inverse => "inverse",
        }
    }
}

// Base URL
pub const BYBIT_V5_URL: &str = "https://api.bybit.com";

// Paths
pub const INSTRUMENTS_INFO_PATH: &str = "/v5/market/instruments-info";
pub const KLINES_PATH: &str = "/v5/market/kline";
pub const RECENT_TRADE_PATH: &str = "/v5/market/recent-trade";
