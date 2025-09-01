use super::endpoints::{get_public_url, ApiEnvironment, PublicEndpoint};
use bytes::Bytes;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct DriftClient {
    client: Client,
    environment: ApiEnvironment,
}

impl DriftClient {
    /// Creates a new Drift API client for the specified environment.
    pub fn new(environment: ApiEnvironment) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to create reqwest client");

        Self { client, environment }
    }

    /// Retrieves information for all available contracts (markets).
    /// This single endpoint provides data for both market lists and market stats.
    /// GET /contracts
    pub async fn get_contracts(&self) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::Contracts, self.environment);
        let res = self.client.get(&url).send().await?;
        res.error_for_status()?.bytes().await
    }

    /// Retrieves historical funding rates for a specific market.
    /// Note: The Drift API does not seem to support `startTime` or `endTime` parameters.
    /// This function will fetch the available history for the application to filter.
    /// GET /fundingRates?marketName={marketName}
    pub async fn get_funding_rates(
        &self,
        market_name: &str,
        // Parameters are kept for a consistent interface but are not used in the request.
        _start_time_ms: Option<u64>,
        _end_time_ms: Option<u64>,
    ) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(
            PublicEndpoint::FundingRates(market_name.to_string()),
            self.environment,
        );
        let res = self.client
            .get(&url)
            .query(&[("marketName", market_name)])
            .send()
            .await?;
        res.error_for_status()?.bytes().await
    }
}