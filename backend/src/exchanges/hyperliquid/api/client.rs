use super::endpoints::{get_public_url, ApiEnvironment, PublicEndpoint};
use bytes::Bytes;
use reqwest::Client;
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct HyperliquidClient {
    client: Client,
    environment: ApiEnvironment,
}

impl HyperliquidClient {
    /// Creates a new `HyperliquidClient` instance.
    ///
    /// The client is configured to use a specific `ApiEnvironment` (e.g., Mainnet, Testnet).
    pub fn new(environment: ApiEnvironment) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to create reqwest client");

        Self { client, environment }
    }

    /// Internal helper method to make a POST request to the public info endpoint.
    ///
    /// This method sends a JSON payload and returns the response as `Bytes`.
    async fn post_info<T: serde::Serialize>(&self, payload: &T) -> Result<Bytes, reqwest::Error> {
        let url = get_public_url(PublicEndpoint::Info, self.environment);
        // The .json() method requires the "json" feature to be enabled for reqwest in Cargo.toml.
        let res = self.client.post(&url).json(payload).send().await?;
        res.error_for_status()?.bytes().await
    }

    /// Retrieves perpetuals metadata (universe and margin tables).
    ///
    /// POST /info with { "type": "meta", "dex": "<optional>" }
    pub async fn get_perp_meta(&self, dex: Option<&str>) -> Result<Bytes, reqwest::Error> {
        let mut body = Map::new();
        body.insert("type".to_string(), Value::String("meta".to_string()));
        if let Some(d) = dex {
            body.insert("dex".to_string(), Value::String(d.to_string()));
        }
        self.post_info(&Value::Object(body)).await
    }

    /// Retrieves perpetuals asset contexts (mark price, current funding, OI, etc.).
    ///
    /// POST /info with { "type": "metaAndAssetCtxs", "dex": "<optional>" }
    pub async fn get_meta_and_asset_ctxs(&self, dex: Option<&str>) -> Result<Bytes, reqwest::Error> {
        let mut body = Map::new();
        body.insert("type".to_string(), Value::String("metaAndAssetCtxs".to_string()));
        if let Some(d) = dex {
            body.insert("dex".to_string(), Value::String(d.to_string()));
        }
        self.post_info(&Value::Object(body)).await
    }

    /// Retrieves historical funding rates.
    ///
    /// POST /info with { "type": "fundingHistory", "coin": "ETH", "startTime": <ms>, "endTime": <ms?> }
    pub async fn get_funding_history(
        &self,
        coin: &str,
        start_time_ms: u64,
        end_time_ms: Option<u64>,
    ) -> Result<Bytes, reqwest::Error> {
        let mut body = Map::new();
        body.insert("type".to_string(), Value::String("fundingHistory".to_string()));
        body.insert("coin".to_string(), Value::String(coin.to_string()));
        body.insert("startTime".to_string(), Value::from(start_time_ms));
        if let Some(end) = end_time_ms {
            body.insert("endTime".to_string(), Value::from(end));
        }
        self.post_info(&Value::Object(body)).await
    }
}
