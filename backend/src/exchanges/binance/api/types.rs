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


#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct BinanceTrade {
    pub id: i64,
    pub price: String,
    pub qty: String,
    pub quoteQty: String,
    pub time: i64,
    pub isBuyerMaker: bool,
}