#[derive(Debug, Clone, Copy)]
pub enum ApiEnvironment {
    Mainnet,
    Testnet,
}

#[derive(Debug, Clone)]
pub enum PublicEndpoint {
    Info,
    Exchange,
}

fn base_url(env: ApiEnvironment) -> &'static str {
    match env {
        ApiEnvironment::Mainnet => "https://api.hyperliquid.xyz",
        ApiEnvironment::Testnet => "https://api.hyperliquid-testnet.xyz",
    }
}

pub fn get_public_url(endpoint: PublicEndpoint, environment: ApiEnvironment) -> String {
    let base = base_url(environment);
    match endpoint {
        PublicEndpoint::Info => format!("{}/info", base),
        PublicEndpoint::Exchange => format!("{}/exchange", base),
    }
}

