// src/exchanges/bybit/api/client.rs
use crate::exchanges::bybit::api::endpoints::{
    BYBIT_V5_URL, Category, INSTRUMENTS_INFO_PATH, KLINES_PATH, RECENT_TRADE_PATH,
};
use reqwest::Client;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct BybitClient {
    client: Client,
    base_url: String,
}

impl BybitClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: BYBIT_V5_URL.to_string(),
        }
    }

    /// Get recent trades (no time range support)
    pub async fn get_recent_trades(&self, category: Category, symbol: &str, limit: Option<usize>) -> Result<Bytes, reqwest::Error> {
        let url = format!("{}{}", self.base_url, RECENT_TRADE_PATH);
        
        let mut query = vec![("category", category.as_str().to_string()), ("symbol", symbol.to_string())];
        if let Some(l) = limit {
            query.push(("limit", l.to_string()));
        }

        let res = self.client.get(&url).query(&query).send().await?;
        res.error_for_status()?.bytes().await
    }

    /// Get historical trades with time range support
    pub async fn get_historical_trades(
        &self,
        category: Category,
        symbol: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<usize>
    ) -> Result<Bytes, reqwest::Error> {
        let url = format!("{}/v5/market/trade", self.base_url);
        
        let mut query = vec![("category", category.as_str().to_string()), ("symbol", symbol.to_string())];
        if let Some(st) = start_time {
            query.push(("start", st.to_string()));
        }
        if let Some(et) = end_time {
            query.push(("end", et.to_string()));
        }
        if let Some(l) = limit {
            query.push(("limit", l.to_string()));
        }

        let res = self.client.get(&url).query(&query).send().await?;
        res.error_for_status()?.bytes().await
    }

    /// Get klines with optional time range
    pub async fn get_klines(
        &self, 
        category: Category, 
        symbol: &str, 
        interval: &str, 
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<usize>
    ) -> Result<Bytes, reqwest::Error> {
        let url = format!("{}{}", self.base_url, KLINES_PATH);
        
        let mut query = vec![
            ("category", category.as_str().to_string()), 
            ("symbol", symbol.to_string()), 
            ("interval", interval.to_string())
        ];
        if let Some(st) = start_time {
            query.push(("start", st.to_string()));
        }
        if let Some(et) = end_time {
            query.push(("end", et.to_string()));
        }
        if let Some(l) = limit {
            query.push(("limit", l.to_string()));
        }

        let res = self.client.get(&url).query(&query).send().await?;
        res.error_for_status()?.bytes().await
    }

    pub async fn get_instruments_info(&self, category: Category, symbol: Option<&str>) -> Result<Bytes, reqwest::Error> {
        let url = format!("{}{}", self.base_url, INSTRUMENTS_INFO_PATH);
        
        let mut query = vec![("category", category.as_str().to_string())];
        if let Some(s) = symbol {
            query.push(("symbol", s.to_string()));
        }

        let res = self.client.get(&url).query(&query).send().await?;
        res.error_for_status()?.bytes().await
    }
}

impl Default for BybitClient {
    fn default() -> Self {
        Self::new()
    }
}