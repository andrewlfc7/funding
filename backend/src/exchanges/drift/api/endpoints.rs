#[derive(Debug, Clone, Copy)]
pub enum ApiEnvironment {
    Mainnet,
    // Testnet is included for future use, assuming a different URL.
    Testnet,
}

#[derive(Debug, Clone)]
pub enum PublicEndpoint {
    // GET /contracts
    Contracts,
    // GET /fundingRates?marketName={marketName}
    FundingRates(String),
}

fn base_url(env: ApiEnvironment) -> &'static str {
    match env {
        ApiEnvironment::Mainnet => "https://data.api.drift.trade",
        // Placeholder for a potential testnet URL
        ApiEnvironment::Testnet => "https://data.api.drift.trade",
    }
}

pub fn get_public_url(endpoint: PublicEndpoint, environment: ApiEnvironment) -> String {
    let base = base_url(environment);
    match endpoint {
        PublicEndpoint::Contracts => format!("{}/contracts", base),
        PublicEndpoint::FundingRates(_) => format!("{}/fundingRates", base),
    }
}