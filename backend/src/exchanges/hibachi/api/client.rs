use super::endpoints::{ApiEnvironment, PublicEndpoint, get_public_url};
use bytes::Bytes;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct HibachiClient {
    client: Client,
    environment: ApiEnvironment,
}

impl HibachiClient {
    /// Creates a new Hibachi API client for the specified environment.
    pub fn new(environment: ApiEnvironment) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to create reqwest client");

        Self {
            client,
            environment,
        }
    }

    /// Retrieves the exchange information, including all available future contracts.
    /// GET /market/exchange-info
    pub async fn get_exchange_info(&self) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::ExchangeInfo, self.environment);
        let res = self.client.get(&url).send().await?;
        res.error_for_status()?.bytes().await
    }

    /// Retrieves the open interest for a specific perpetual market.
    /// GET /market/data/open-interest?symbol={symbol}
    pub async fn get_open_interest(&self, symbol: &str) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(
            PublicEndpoint::OpenInterest(symbol.to_string()),
            self.environment,
        );
        let res = self
            .client
            .get(&url)
            .query(&[("symbol", symbol)])
            .send()
            .await?;
        res.error_for_status()?.bytes().await
    }

    /// Retrieves 24-hour statistics for a specific perpetual market.
    /// GET /market/data/stats?symbol={symbol}
    pub async fn get_stats(&self, symbol: &str) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::Stats(symbol.to_string()), self.environment);
        let res = self
            .client
            .get(&url)
            .query(&[("symbol", symbol)])
            .send()
            .await?;
        res.error_for_status()?.bytes().await
    }

    /// Retrieves various prices for a specific perpetual market.
    /// GET /market/data/prices?symbol={symbol}
    pub async fn get_prices(&self, symbol: &str) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::Prices(symbol.to_string()), self.environment);
        let res = self
            .client
            .get(&url)
            .query(&[("symbol", symbol)])
            .send()
            .await?;
        res.error_for_status()?.bytes().await
    }
}
