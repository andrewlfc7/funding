// src/exchanges/binance/api/client.rs
use crate::exchanges::binance::api::endpoints::{
    EXCHANGE_INFO_PATH, FUTURES_EXCHANGE_INFO_PATH, FUTURES_KLINES_PATH, FUTURES_TRADES_PATH,
    KLINES_PATH, TRADES_PATH, USD_FUTURES_API_URL, SPOT_API_URL, MarketType,
};
use reqwest::Client;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct BinanceClient {
    client: Client,
}

impl BinanceClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Get recent trades (doesn't support time range - use get_historical_trades for time ranges)
    pub async fn get_trades(&self, market_type: MarketType, symbol: &str, limit: usize) -> Result<Bytes, reqwest::Error> {
        let (base_url, path) = match market_type {
            MarketType::Spot => (SPOT_API_URL, TRADES_PATH),
            MarketType::UsdFutures => (USD_FUTURES_API_URL, FUTURES_TRADES_PATH),
        };
        let url = format!("{}{}", base_url, path);

        let res = self.client.get(&url)
            .query(&[("symbol", symbol), ("limit", &limit.to_string())])
            .send().await?;

        res.error_for_status()?.bytes().await
    }

    /// Get historical aggregate trades with time range support
    pub async fn get_historical_trades(
        &self, 
        market_type: MarketType, 
        symbol: &str, 
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<usize>
    ) -> Result<Bytes, reqwest::Error> {
        let (base_url, path) = match market_type {
            MarketType::Spot => (SPOT_API_URL, "/api/v3/aggTrades"),
            MarketType::UsdFutures => (USD_FUTURES_API_URL, "/fapi/v1/aggTrades"),
        };
        let url = format!("{}{}", base_url, path);

        let mut query = vec![("symbol", symbol.to_string())];
        if let Some(st) = start_time {
            query.push(("startTime", st.to_string()));
        }
        if let Some(et) = end_time {
            query.push(("endTime", et.to_string()));
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
        market_type: MarketType, 
        symbol: &str, 
        interval: &str, 
        start_time: Option<u64>,
        end_time: Option<u64>,
        limit: Option<usize>
    ) -> Result<Bytes, reqwest::Error> {
        let (base_url, path) = match market_type {
            MarketType::Spot => (SPOT_API_URL, KLINES_PATH),
            MarketType::UsdFutures => (USD_FUTURES_API_URL, FUTURES_KLINES_PATH),
        };
        let url = format!("{}{}", base_url, path);
        
        let mut query = vec![("symbol", symbol.to_string()), ("interval", interval.to_string())];
        if let Some(st) = start_time {
            query.push(("startTime", st.to_string()));
        }
        if let Some(et) = end_time {
            query.push(("endTime", et.to_string()));
        }
        if let Some(l) = limit {
            query.push(("limit", l.to_string()));
        }

        let res = self.client.get(&url).query(&query).send().await?;
        res.error_for_status()?.bytes().await
    }

    pub async fn get_exchange_info(&self, market_type: MarketType) -> Result<Bytes, reqwest::Error> {
        let (base_url, path) = match market_type {
            MarketType::Spot => (SPOT_API_URL, EXCHANGE_INFO_PATH),
            MarketType::UsdFutures => (USD_FUTURES_API_URL, FUTURES_EXCHANGE_INFO_PATH),
        };
        let url = format!("{}{}", base_url, path);

        let res = self.client.get(&url).send().await?;
        res.error_for_status()?.bytes().await
    }
}

impl Default for BinanceClient {
    fn default() -> Self {
        Self::new()
    }
}