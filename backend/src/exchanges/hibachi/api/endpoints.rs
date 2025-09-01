#[derive(Debug, Clone, Copy)]
pub enum ApiEnvironment {
    Mainnet,
    // Testnet variant included for future-proofing, even if URL is unknown.
    Testnet,
}

#[derive(Debug, Clone)]
pub enum PublicEndpoint {

    ExchangeInfo,
    OpenInterest(String),
    Stats(String),
    Prices(String),
}

fn base_url(env: ApiEnvironment) -> &'static str {
    match env {
        ApiEnvironment::Mainnet => "https://data-api.hibachi.xyz",
        ApiEnvironment::Testnet => "https://data-api.hibachi.xyz", // Placeholder URL
    }
}

pub fn get_public_url(endpoint: PublicEndpoint, environment: ApiEnvironment) -> String {
    let base = base_url(environment);
    match endpoint {
        PublicEndpoint::ExchangeInfo => format!("{}/market/exchange-info", base),
        PublicEndpoint::OpenInterest(_) => format!("{}/market/data/open-interest", base),
        PublicEndpoint::Stats(_) => format!("{}/market/data/stats", base),
        PublicEndpoint::Prices(_) => format!("{}/market/data/prices", base),
    }
}

