use super::endpoints::{get_public_url, ApiEnvironment, PublicEndpoint};
use bytes::Bytes;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct BluefinClient {
    client: Client,
    environment: ApiEnvironment,
}

impl BluefinClient {
    /// Creates a new Bluefin API client.
    pub fn new(environment: ApiEnvironment) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to create reqwest client");

        Self { client, environment }
    }

    /// Retrieves exchange information, including all available perpetual markets.
    /// GET /v1/exchange/info
    pub async fn get_exchange_info(&self) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::ExchangeInfo, self.environment);
        let res = self.client.get(&url).send().await?;
        res.error_for_status()?.bytes().await
    }

    /// Retrieves ticker statistics for a single market.
    /// Note: `get_tickers` is generally more efficient for syncing.
    /// GET /v1/exchange/ticker?symbol={symbol}
    pub async fn get_ticker(&self, symbol: &str) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::Ticker(symbol.to_string()), self.environment);
        let res = self.client.get(&url).query(&[("symbol", symbol)]).send().await?;
        res.error_for_status()?.bytes().await
    }

    /// Retrieves ticker statistics for ALL available markets in a single call.
    /// This is the preferred method for syncing market stats.
    /// GET /v1/exchange/tickers
    pub async fn get_tickers(&self) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::Tickers, self.environment);
        let res = self.client.get(&url).send().await?;
        res.error_for_status()?.bytes().await
    }

    /// Retrieves historical funding rates for a specific market.
    /// Note: The Bluefin API does not appear to support `startTime` or `endTime` query parameters.
    /// This function will fetch the available history, and the application logic will need to filter it.
    /// GET /v1/exchange/fundingRateHistory?symbol={symbol}
    pub async fn get_funding_rate_history(
        &self,
        symbol: &str,
        // These parameters are kept for a consistent interface but are not sent in the request.
        _start_time_ms: Option<u64>,
        _end_time_ms: Option<u64>,
    ) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(
            PublicEndpoint::FundingRateHistory(symbol.to_string()),
            self.environment,
        );
        let res = self.client.get(&url).query(&[("symbol", symbol)]).send().await?;
        res.error_for_status()?.bytes().await
    }
}