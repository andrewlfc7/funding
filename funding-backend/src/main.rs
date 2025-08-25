mod exchanges;

use exchanges::paradex::api::{client::ParadexClient, endpoints as paradex_api};
use exchanges::extended::api::{client::ExtendedClient, endpoints as extended_api};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let btc_market = "BTC-USD-PERP";

    // Paradex
    println!("--- Paradex ---");
    let paradex_client = ParadexClient::new(paradex_api::ApiEnvironment::Mainnet);

    // Markets
    let markets = paradex_client.get_markets().await?;
    println!("Paradex Markets:\n{}\n", String::from_utf8_lossy(&markets));

    // Market Summary
    let summary = paradex_client.get_markets_summary(btc_market).await?;
    println!("Paradex Market Summary for {}:\n{}\n", btc_market, String::from_utf8_lossy(&summary));

    // Funding Data (8 hour interval)
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let eight_hours_ago = now - Duration::from_secs(2000 * 60 * 60).as_secs();
    let funding = paradex_client
        .get_funding_data(btc_market, Some(eight_hours_ago), Some(now))
        .await?;
    println!(
        "Paradex Funding Data for {} (last 8 hours):\n{}\n",
        btc_market, String::from_utf8_lossy(&funding)
    );

    // Extended
    println!("--- Extended ---");
    let extended_client = ExtendedClient::new(extended_api::ApiEnvironment::Mainnet);

    // Markets
    let ext_markets = extended_client.get_markets(None).await?;
    println!("Extended Markets:\n{}\n", String::from_utf8_lossy(&ext_markets));

    let btc_market_ex = "BTC-USD";


    // Market Stats
    let ext_stats = extended_client.get_market_stats(btc_market_ex).await?;
    println!("Extended Market Stats for {}:\n{}\n", btc_market_ex, String::from_utf8_lossy(&ext_stats));


    let ext_open_interest = extended_client
        .get_open_interest(btc_market_ex, Some("P1D"), Some(eight_hours_ago), Some(now))
        .await?;
    println!("Extended Open Interest for {}:\n{}\n", btc_market_ex, String::from_utf8_lossy(&ext_open_interest));



    // Funding (8 hour interval)
    let ext_funding = extended_client
        .get_funding(btc_market_ex, Some(eight_hours_ago), Some(now))
        .await?;
    println!(
        "Extended Funding Data for {} (last 8 hours):\n{}\n",
        btc_market_ex, String::from_utf8_lossy(&ext_funding)
    );

    Ok(())
}