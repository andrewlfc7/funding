#[derive(Debug, Clone, Copy)]
pub enum ApiEnvironment {
    Mainnet,
    // Testnet variant included for future-proofing.
    Testnet,
}

#[derive(Debug, Clone)]
pub enum PublicEndpoint {
    // GET /v1/exchange/info
    ExchangeInfo,
    // GET /v1/exchange/ticker?symbol={symbol}
    Ticker(String),
    // GET /v1/exchange/tickers
    Tickers,
    // GET /v1/exchange/fundingRateHistory?symbol={symbol}
    FundingRateHistory(String),
}

fn base_url(env: ApiEnvironment) -> &'static str {
    match env {
        ApiEnvironment::Mainnet => "https://api.sui-prod.bluefin.io",
        // Assuming testnet has a different subdomain, adjust if necessary
        ApiEnvironment::Testnet => "https://api.sui-test.bluefin.io",
    }
}

pub fn get_public_url(endpoint: PublicEndpoint, environment: ApiEnvironment) -> String {
    let base = base_url(environment);
    match endpoint {
        PublicEndpoint::ExchangeInfo => format!("{}/v1/exchange/info", base),
        PublicEndpoint::Ticker(_) => format!("{}/v1/exchange/ticker", base),
        PublicEndpoint::Tickers => format!("{}/v1/exchange/tickers", base),
        PublicEndpoint::FundingRateHistory(_) => format!("{}/v1/exchange/fundingRateHistory", base),
    }
}
