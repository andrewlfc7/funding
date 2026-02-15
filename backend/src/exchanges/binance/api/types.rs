use serde::Deserialize;

// --- Types for Exchange Info ---
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct BinanceExchangeInfoResponse {
    pub symbols: Vec<BinanceSymbol>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct BinanceSymbol {
    pub symbol: String,
    pub status: String,
    pub baseAsset: String,
    pub quoteAsset: String,
    pub contractType: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(non_snake_case)]
pub struct BinanceTrade {
    #[serde(alias = "a")]
    pub id: i64,
    #[serde(alias = "p")]
    pub price: String,
    #[serde(alias = "q")]
    pub qty: String,
    #[serde(default)]
    pub quoteQty: Option<String>, // may be absent on aggTrades
    #[serde(alias = "T")]
    pub time: i64,
    #[serde(alias = "m")]
    pub isBuyerMaker: bool,
}
