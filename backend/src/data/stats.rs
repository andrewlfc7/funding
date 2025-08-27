use crate::exchanges::paradex::api::{
    client::ParadexClient,
    endpoints::ApiEnvironment as ParadexEnv,
};
use crate::exchanges::extended::api::{
    client::ExtendedClient,
    endpoints::ApiEnvironment as ExtendedEnv,
};
use crate::exchanges::paradex::handler::handler::parse_paradex_market_stats;
use crate::exchanges::extended::handler::handler::parse_extended_market_stats;
use crate::db::insert::insert_market_stats;
use sqlx::PgPool;

pub async fn collect_daily_market_stats(pool: &PgPool) -> anyhow::Result<()> {
    let exchanges = sqlx::query!("SELECT id,name FROM exchanges WHERE is_active = true")
        .fetch_all(pool).await?;

    for exch in exchanges {
        let markets = sqlx::query!(
            "SELECT id, market_symbol FROM markets WHERE exchange_id=$1 AND is_active=true",
            exch.id
        ).fetch_all(pool).await?;

        for m in markets {
            match exch.name.as_str() {
                "paradex" => {
                    let client = ParadexClient::new(ParadexEnv::Mainnet);
                    let raw = client.get_markets_summary(&m.market_symbol).await?;
                    let stats = parse_paradex_market_stats(&raw)?;
                    for s in stats {
                        insert_market_stats(pool, m.id, &s).await?;
                    }
                }
                "extended" => {
                    let client = ExtendedClient::new(ExtendedEnv::Mainnet);
                    let raw = client.get_market_stats(&m.market_symbol).await?;
                    let stat = parse_extended_market_stats(&raw, &m.market_symbol)?;
                    insert_market_stats(pool, m.id, &stat).await?;
                }
                _ => {}
            }
        }
    }
    Ok(())
}