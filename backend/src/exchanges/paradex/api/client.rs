use super::endpoints::{ApiEnvironment, PublicEndpoint, get_public_url};
use bytes::Bytes;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct ParadexClient {
    client: Client,
    environment: ApiEnvironment,
}

impl ParadexClient {
    pub fn new(environment: ApiEnvironment) -> Self {
        Self {
            client: Client::new(),
            environment,
        }
    }

    pub async fn get_markets(&self) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::Markets, self.environment);
        let res = self.client.get(&url).send().await?;
        res.error_for_status()?.bytes().await
    }
    
    pub async fn get_markets_summary(&self, market: &str) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::MarketsSummary, self.environment);
        let res = self
            .client
            .get(&url)
            .query(&[("market", market)])
            .send()
            .await?;
        res.error_for_status()?.bytes().await
    }

    pub async fn get_funding_data(
        &self,
        market: &str,
        start_time: Option<u64>,
        end_time: Option<u64>,
        cursor: Option<&str>, 
    ) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::FundingData, self.environment);

        let mut query_params = vec![("market".to_string(), market.to_string())];
        
        if let Some(st) = start_time {
            query_params.push(("start_at".to_string(), st.to_string()));
        }
        if let Some(et) = end_time {
            query_params.push(("end_at".to_string(), et.to_string()));
        }
        if let Some(c) = cursor {
            query_params.push(("cursor".to_string(), c.to_string()));
        }

        let res = self
            .client
            .get(&url)
            .query(&query_params)
            .send()
            .await?;

        res.error_for_status()?.bytes().await
    }

}