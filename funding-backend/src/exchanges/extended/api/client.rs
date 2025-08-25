// 
use super::endpoints::{ApiEnvironment, PublicEndpoint, get_public_url};
use bytes::Bytes;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct ExtendedClient {
    client: Client,
    environment: ApiEnvironment,
}

impl ExtendedClient {
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


    pub async fn get_markets(&self, market: Option<&str>) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::Markets, self.environment);
        let mut request = self.client.get(&url);
        if let Some(market) = market {
            request = request.query(&[("market", market)]);
        }
        let res = request.send().await?;
        res.error_for_status()?.bytes().await
    }
    

    pub async fn get_market_stats(&self, market: &str) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::MarketStats(market.to_string()), self.environment);
        let res = self.client.get(&url).send().await?;
        res.error_for_status()?.bytes().await
    }


    pub async fn get_open_interest(
        &self,
        market: &str,
        interval: Option<&str>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::OpenInterest(market.to_string()), self.environment);

        let mut query_params = Vec::new();
        if let Some(interval) = interval {
            query_params.push(("interval".to_string(), interval.to_string()));
        }
        if let Some(st) = start_time {
            query_params.push(("startTime".to_string(), st.to_string()));
        }
        if let Some(et) = end_time {
            query_params.push(("endTime".to_string(), et.to_string()));
        }

        let mut request_builder = self.client.get(&url);
        if !query_params.is_empty() {
            request_builder = request_builder.query(&query_params);
        }

        let res = request_builder.send().await?;
        res.error_for_status()?.bytes().await
    }


    pub async fn get_funding(
        &self,
        market: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::Funding(market.to_string()), self.environment);

        let mut query_params = Vec::new();
        if let Some(st) = start_time {
            query_params.push(("startTime".to_string(), st.to_string()));
        }
        if let Some(et) = end_time {
            query_params.push(("endTime".to_string(), et.to_string()));
        }

        let mut request_builder = self.client.get(&url);
        if !query_params.is_empty() {
            request_builder = request_builder.query(&query_params);
        }

        let res = request_builder.send().await?;
        res.error_for_status()?.bytes().await
    }
}