// src/exchanges/extended/api/endpoints.rs

use std::borrow::Cow;

pub const EXTENDED_API_ENDPOINTS: ExtendedAPIEndpoints = ExtendedAPIEndpoints {
    base_url: "https://api.extended.exchange/api/v1",
    testnet_base_url: "https://api.starknet.sepolia.extended.exchange/api/v1",
};

#[derive(Debug, Clone, Copy)]
pub struct ExtendedAPIEndpoints {
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
        ApiEnvironment::Mainnet => EXTENDED_API_ENDPOINTS.base_url,
        ApiEnvironment::Testnet => EXTENDED_API_ENDPOINTS.testnet_base_url,
    };
    format!("{}{}", base, endpoint.to_path())
}

#[derive(Debug, Clone)]
pub enum PublicEndpoint {
    Markets,

    MarketStats(String),

    OpenInterest(String),

    Funding(String),
}


impl PublicEndpoint {
    pub fn to_path(&self) -> Cow<'static, str> {
        match self {
            Self::Markets => Cow::from("/info/markets"),
            Self::MarketStats(market) => Cow::from(format!("/info/markets/{}/stats", market)),
            Self::OpenInterest(market) => Cow::from(format!("/info/{}/open-interests", market)),
            Self::Funding(market) => Cow::from(format!("/info/{}/funding", market)),
        }
    }
}
