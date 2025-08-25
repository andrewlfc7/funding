// src/exchanges/paradex/api/endpoints.rs

use std::borrow::Cow;

pub const PARADEX_API_ENDPOINTS: ParadexAPIEndpoints = ParadexAPIEndpoints {
    base_url: "https://api.prod.paradex.trade/v1",
    testnet_base_url: "https://api.testnet.paradex.trade/v1",
};

#[derive(Debug, Clone, Copy)]
pub struct ParadexAPIEndpoints {
    pub base_url: &'static str,
    pub testnet_base_url: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiEnvironment {
    Mainnet,
    Testnet,
}

pub fn get_public_url(endpoint: PublicEndpoint, environment: ApiEnvironment) -> String {
    let base = match environment {
        ApiEnvironment::Mainnet => PARADEX_API_ENDPOINTS.base_url,
        ApiEnvironment::Testnet => PARADEX_API_ENDPOINTS.testnet_base_url,
    };
    format!("{}{}", base, endpoint.to_path())
}

#[derive(Debug, Clone)]
pub enum PublicEndpoint {
    Markets,

    MarketsSummary,

    FundingData,

}



impl PublicEndpoint {
    pub fn to_path(&self) -> Cow<'static, str> {
        match self {
            Self::Markets => Cow::from("/markets"),
            Self::MarketsSummary => Cow::from("/markets/summary"),
            Self::FundingData => Cow::from("/funding/data"),
        }
    }
}