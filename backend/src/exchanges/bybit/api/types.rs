use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BybitInstrumentsResponse {
    pub result: BybitInstrumentsResult,
}
#[derive(Debug, Deserialize)]
pub struct BybitInstrumentsResult {
    pub list: Vec<BybitSymbol>,
}
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct BybitSymbol {
    pub symbol: String,
    pub status: String,
    pub baseCoin: String,
    pub quoteCoin: String,
}

#[derive(Debug, Deserialize)]
pub struct BybitKlineResponse {
    pub result: BybitKlineResult,
}
#[derive(Debug, Deserialize)]
pub struct BybitKlineResult {
    pub list: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct BybitTradeResponse {
    pub result: BybitTradeResult,
}
#[derive(Debug, Deserialize)]
pub struct BybitTradeResult {
    pub list: Vec<BybitTrade>,
}
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct BybitTrade {
    pub execId: String,
    pub price: String,
    pub size: String,
    pub side: String,
    pub time: String,
}
